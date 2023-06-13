use std::{ net::SocketAddr, fs::{read_to_string, File}, io::Read, error::Error, env::current_exe};
use axum::{Router, routing::{get, post, put, delete}, response::{IntoResponse, Redirect}, http::{Response, StatusCode}, extract::{Path, State}, Json};
use hyper::{Body};
use mongodb::{bson::{doc, Document, to_bson, oid::ObjectId}, options::{ClientOptions, ServerApi, ServerApiVersion, ResolverConfig}, Client, Collection, Database};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use futures::{stream::{StreamExt, TryStreamExt}, TryFutureExt};

#[derive(Debug, Deserialize)]
struct CreateUserParams {
    email: String,
    user_name: String
}

#[derive(Clone)]
struct AppState {
    db: Database
}

#[derive(Deserialize, Debug, Serialize)]
struct UserCollection {
    _id: ObjectId,
    user_name: String,
    email: String
}

#[derive(Deserialize, Debug, Serialize)]
struct UpdateUserParams {
    user_name: String,
    email: String
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    
    dotenv().ok();
    
    let mongo_uri = std::env::var("MONGO_URI").expect("MONGO_URI must be set.");
    
    let options = ClientOptions::parse_with_resolver_config(mongo_uri, ResolverConfig::cloudflare()).await?;
    
    let client = Client::with_options(options)?;
    
    // let app = app.fallback(handle_404);
    
    let db = client.database("cse_312");
    
    
    let state = AppState {
        db
    };

    let app = Router::new()
    .route("/", get(root))
    .route("/style.css", get(get_styles))
    .route("/index.js", get(get_js))
    .route("/hello", get(get_hello))
    .route("/hi", get(hello_redirect))
    .route("/image/:image_name", get(get_image))
    .route("/users", post(create_user))
    .route("/users", get(get_users))
    .route("/users/:id", get(get_user))
    .route("/users/:id", put(update_user))
    .route("/users/:id", delete(delete_user))
    .with_state(state);

    let addr = SocketAddr::from(([0,0,0,0], 8080));
    axum::Server::bind(&addr)
    .serve(app.into_make_service())
    .await
    .unwrap();

    Ok(())
}

async fn delete_user(State(state): State<AppState>, Path(id): Path<String>) -> Response<Body> {
    let ooid = match ObjectId::parse_str(&id) {
        Ok(id) => id,
        Err(_) => {
            return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("INVALID ID"))
            .unwrap();
        }
    };

    let res = state.db.collection::<UserCollection>("users").delete_one(doc! {"_id": ooid}, None).await;

    match res {
        Ok(_) => {
            return Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(Body::from("DELETED"))
            .unwrap();
        },
        Err(_) => {
            return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("NOT FOUND"))
            .unwrap();
        }
    }
}

async fn update_user(State(state): State<AppState>,Path(id): Path<String>, body: Json<UpdateUserParams>) -> Response<Body> {
    
    let ooid = match ObjectId::parse_str(&id) {
        Ok(id) => id,
        Err(_) => {
            return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("INVALID ID"))
            .unwrap();
        }
    };
    
    let res = state.db.collection::<UserCollection>("users").update_one(
    doc! {"_id": ooid}, doc! {"$set": {"user_name": &body.user_name, "email": &body.email, "_id": ooid}},None).await;

        match res {
            Ok(_) => {
                let res = state.db.collection::<UserCollection>("users").find_one(doc! {"_id": ooid}, None).await;

                match res {
                    Ok(user) => {
                        if let Some(res_user) = user {
                            return Response::builder()
                            .status(StatusCode::OK)
                            .body(Body::from(serde_json::to_string(&res_user).unwrap()))
                            .unwrap();
                        } else {
                            return Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .body(Body::from("UPDATED USER NOT FOUND "))
                            .unwrap();
                        }
                    },
                    Err(_) => {
                        return Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(Body::from("UPDATED USER NOT FOUND "))
                        .unwrap();
                    }   
                }


            },
            Err(_) => {
                return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("USER NOT FOUND "))
                .unwrap();
            }
        }
}

async fn get_user(State(state): State<AppState>,Path(id): Path<String>) -> Response<Body> {
    
    let ooid = match ObjectId::parse_str(&id) {
        Ok(id) => id,
        Err(_) => {
            return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("INVALID ID"))
            .unwrap();
        }
    };
    
    let res = state.db.collection::<UserCollection>("users").find_one(
    doc! {"_id": ooid}, None).await;

        match res {
            Ok(user) => {
                match user {
                    Some(user_res) => {
                        println!("{:?}", user_res);
                        let final_res = serde_json::to_string(&user_res).unwrap();

                        return Response::builder()
                        .status(StatusCode::OK)
                        .body(Body::from(final_res))
                        .unwrap();
                    },
                    None => {
                        return Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(Body::from("USER NOT FOUND"))
                        .unwrap();
                    }
                }

            },
            Err(_) => {
                return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("USER NOT FOUND "))
                .unwrap();
            }
        }
}

async fn get_users(State(state): State<AppState>) -> Response<Body> {
    let res = state.db.collection::<UserCollection>("users").find(None, None).await;

    let mut res_collection: Vec<UserCollection> = Vec::new();

    match res {
        Ok(mut cursor) => {
            while let Some(doc) = cursor.next().await {
                if let Ok(user) = doc {
                    res_collection.push(user);
                } 
              }

            let final_res = serde_json::to_string(&res_collection);
            
            return Response::builder()
            .status(StatusCode::OK)
            .body(Body::from(final_res.unwrap()))
            .unwrap();
        },
        Err(_) => {
            return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("CANNOT GET USERS"))
            .unwrap();
        }
    }
}

async fn create_user(State(state): State<AppState>,body: Json<CreateUserParams>) -> Response<Body> {
    let email = &body.email;
    let user_name = &body.user_name;
    
    let insert_doc = doc! {
        "user_name": user_name,
        "email": email
    };
        
    let res = state.db.collection("users").insert_one(insert_doc, None).await;

    match res {
        Ok(data) => {
            let insert_id = data.inserted_id;

            let user = state.db.collection::<UserCollection>("users").find_one(doc! {"_id": insert_id}, None).await;

            match user {
                Ok(data) => {
                    match data {
                        Some(response) => {
                            let json_doc = serde_json::to_string(&response).unwrap();

                            return Response::builder()
                            .status(StatusCode::CREATED)
                            .body(Body::from(json_doc))
                            .unwrap();
                        },
                        None => {
                            return Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(Body::from("ERROR FINDING CREATED USER"))
                            .unwrap();
                        }
                    }
                },
                Err(_) => {
                    return Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from("ERROR FINDING CREATED USER"))
                    .unwrap();
                }
            }
        },

        Err(_) => {
            return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("ERROR CREATING USER"))
            .unwrap();
        }
    }
    
}

async fn get_image(Path(image_name): Path<String>) -> Response<Body> {
    let file = File::open("src/image/".to_owned() + &image_name);

    let res = match file {
        Ok(mut image) => {
            let mut buf = Vec::new();

            image.read_to_end(&mut buf).unwrap();

            Response::builder()
            .header("Content-Type", "image/jpeg")
            .status(StatusCode::OK)
            .body(Body::from(buf))
            .unwrap()
        },
        Err(_) => {
            Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("IMAGE NOT FOUND"))
            .unwrap()
        }
    };

    res

}

async fn root() -> Response<Body> {
    let content = read_to_string("src/html/index.html").expect("No index HTML!");

    Response::builder()
    .header("Content-Type", "text/html; charset=utf-8")
    .header("Content-Length", content.len())
    .header("X-Content-Type-Options", "nosniff")
    .status(StatusCode::OK)
    .body(Body::from(content))
    .unwrap()
}

async fn get_styles() -> Response<Body> {
    let content = read_to_string("src/css/index.css").expect("No index CSS!");

    Response::builder()
    .header("Content-Type", "text/css; charset=utf-8")
    .header("Content-Length", content.len())
    .header("X-Content-Type-Options", "nosniff")
    .status(StatusCode::OK)
    .body(Body::from(content))
    .unwrap()
}

async fn get_js() -> Response<Body> {
    let content = read_to_string("src/js/index.js").expect("No index JS!");

    Response::builder()
    .header("Content-Type", "application/javascript; charset=utf-8")
    .header("Content-Length", content.len())
    .header("X-Content-Type-Options", "nosniff")
    .status(StatusCode::OK)
    .body(Body::from(content))
    .unwrap()
}

async fn get_hello() -> Response<Body> {
    let content = "hello";

    Response::builder()
    .header("Content-Type", "text/plain; charset=utf-8")
    .header("Content-Length", content.len())
    .header("X-Content-Type-Options", "nosniff")
    .status(StatusCode::OK)
    .body(Body::from(content))
    .unwrap()
}

async fn hello_redirect() -> impl IntoResponse {

    Redirect::permanent("/hello")
}

// async fn handle_404() -> impl IntoResponse {
//     let body = "The Content Could not be found";

//     let res = Response::builder()
//     .version(Version::HTTP_11)
//     .status(StatusCode::NOT_FOUND)
//     .header("Content-Type", "text/plain")
//     .header("Content-Length", body.len())
//     .body(body::boxed(http_body::Full::from(body)))
//     .unwrap();

//     res
// }