use std::{error::Error as StdError};

use futures::{stream::FuturesOrdered, StreamExt};
use hyper::Uri;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    http::{
        Request,
        Response,
        error::{Error, Result},
        response::JsonResponseBuilder,
        body::json_from_body,
    }, 
    state::AppState,
    components::{auth::{require_session, login_redirect}, html::{check_if_html_headers, response_index_html_parts}}, 
    db::record::EventListener, 
    event
};

pub async fn handle_get(state: AppState, req: Request) -> Result<Response> {
    let conn = state.db.pool.get().await?;
    let session_check = require_session(&*conn, req.headers()).await;

    if check_if_html_headers(req.headers())? {
        return match session_check {
            Ok(_) => response_index_html_parts(state.template),
            Err(_) => login_redirect(req.uri())
        }
    }

    let (user, _) = session_check?;

    JsonResponseBuilder::new(200)
        .payload_response(EventListener::find_user_id(&*conn, &user.id).await?)
}

#[derive(Debug, Deserialize)]
struct NewEventListener {
    event_name: String,
    endpoint: String,

    ref_table: String,
    ref_id: i64
}

#[derive(Debug, Serialize)]
struct InvalidEndpoint {
    endpoint: String,
    reason: String
}

#[derive(Debug, Serialize)]
struct InvalidEventName {
    event_name: String,
    reason: String
}

#[derive(Debug, Serialize)]
struct InvalidRefTable {
    ref_table: String,
    reason: String
}

fn check_uri(endpoint: String, uri: Uri) -> std::result::Result<(), InvalidEndpoint> {
    if let Some(scheme) = uri.scheme() {
        match scheme.as_str() {
            "http" | "https" => {},
            _ => {
                return Err(InvalidEndpoint {
                    endpoint,
                    reason: "invalid scheme, must be http or https".into()
                });
            }
        }
    } else {
        return Err(InvalidEndpoint {
            endpoint,
            reason: "no scheme specified for the given endpoing, must be http or https".into()
        });
    }

    Ok(())
}

pub async fn handle_post(state: AppState, req: Request) -> Result<Response> {
    let mut conn = state.db.pool.get().await?;
    let (user, _) = require_session(&*conn, req.headers()).await?;
    let body = req.into_body();

    let new_listeners: Vec<NewEventListener> = json_from_body(body).await?;
    let mut invalid_event_name: Vec<InvalidEventName> = Vec::with_capacity(new_listeners.len());
    let mut invalid_endpoint: Vec<InvalidEndpoint> = Vec::with_capacity(new_listeners.len());
    let mut invalid_ref_table: Vec<InvalidRefTable> = Vec::with_capacity(new_listeners.len());
    let mut invalid_ref_id: Vec<(String, i64)> = Vec::with_capacity(new_listeners.len());
    let mut to_create: Vec<EventListener> = Vec::with_capacity(new_listeners.len());
    let mut failed_check;

    for listener in new_listeners {
        failed_check = false;

        if let Ok(uri) = listener.endpoint.parse::<Uri>() {
            match check_uri(listener.endpoint.clone(), uri) {
                Ok(()) => {},
                Err(err) => {
                    invalid_endpoint.push(err);
                    failed_check = true;
                }
            }
        } else {
            invalid_endpoint.push(InvalidEndpoint {
                endpoint: listener.endpoint.clone(),
                reason: "failed to parse as a valid uri".into()
            });
            failed_check = true;
        }

        match listener.event_name.as_str() {
            event::name::FS_ITEM_CREATED |
            event::name::FS_ITEM_DELETED |
            event::name::FS_ITEM_SYNCED |
            event::name::FS_ITEM_UPDATED => {},
            _ => {
                invalid_event_name.push(InvalidEventName {
                    event_name: listener.event_name.clone(),
                    reason: "unknown event name given".into()
                });
                failed_check = true;
            }
        }

        match listener.ref_table.as_str() {
            "fs_items" => {},
            _ => {
                invalid_ref_table.push(InvalidRefTable {
                    ref_table: listener.ref_table.clone(),
                    reason: "unknown ref table given".into()
                });
                failed_check = true;
            }
        }

        let mut query = String::with_capacity(listener.ref_table.len() + 29);
        query.push_str("select id from ");
        query.push_str(&listener.ref_table);
        query.push_str(" where id = $1");

        let check = conn.query_opt(query.as_str(), &[&listener.ref_id]).await?;

        if check.is_none() {
            invalid_ref_id.push((listener.ref_table.clone(), listener.ref_id.clone()));
            failed_check = true;
        }

        if !failed_check {
            to_create.push(EventListener {
                id: uuid::Uuid::new_v4(),
                event_name: listener.event_name,
                endpoint: listener.endpoint,
                ref_table: listener.ref_table,
                ref_id: listener.ref_id,
                users_id: user.id
            });
        }
    }

    if !invalid_endpoint.is_empty() || !invalid_event_name.is_empty() || !invalid_ref_table.is_empty() || !invalid_ref_id.is_empty() {
        return JsonResponseBuilder::new(400)
            .set_message("one or more given events are invalid")
            .payload_response(json!({
                "invalid_event_name": invalid_event_name,
                "invalid_endpoint": invalid_endpoint,
                "invalid_ref_table": invalid_ref_table,
                "invalid_ref_id": invalid_ref_id
            }));
    }

    let transaction = conn.transaction().await?;

    for record in to_create.iter_mut() {
        loop {
            if let Err(err) = record.insert(&transaction).await {
                if let Some(_db_error) = err.source().and_then(|e| e.downcast_ref::<tokio_postgres::error::DbError>()) {
                    return Err(err);
                } else {
                    return Err(err);
                }
            } else {
                break;
            }
        }
    }

    transaction.commit().await?;

    JsonResponseBuilder::new(200).payload_response(to_create)
}

pub async fn handle_delete(state: AppState, req: Request) -> Result<Response> {
    let mut conn = state.db.pool.get().await?;
    let (user, _) = require_session(&*conn, req.headers()).await?;
    let body = req.into_body();
    let mut failed = false;
    let id_list: Vec<uuid::Uuid> = json_from_body(body).await?;
    let mut unknown = Vec::with_capacity(id_list.len());
    let mut error_list = String::new();
    let transaction = conn.transaction().await?;
    
    {
        let mut query_list = Vec::with_capacity(id_list.len());

        {
            let user_id_str = user.id.to_string();
            let mut outbound_query = FuturesOrdered::new();

            for uuid in id_list.iter() {
                let uuid_str = uuid.to_string();
                let mut query = String::new();
                query.push_str("delete from event_listeners where id = ");
                query.push_str(&uuid_str);
                query.push_str(" where users_id = ");
                query.push_str(&user_id_str);
    
                query_list.push(query);
            }
    
            for q in 0..query_list.len() {
                outbound_query.push(transaction.execute(query_list[q].as_str(), &[]));
            }
    
            let mut id_iter = id_list.iter();
    
            while let Some(res) = outbound_query.next().await {
                match res {
                    Ok(amount) => {
                        if amount != 1 {
                            unknown.push(id_iter.next().unwrap().clone());
                        }
                    },
                    Err(err) => {
                        let err_str = err.to_string();

                        if failed {
                            error_list.push('\n');
                        } else {
                            failed = true;
                        }

                        error_list.push_str(&err_str);
                    }
                }
            }
        }
    }

    if failed {
        transaction.rollback().await?;

        return Err(Error::new_source(500, "DatabaseError", "database error when processing request", error_list));
    }

    if unknown.len() != 0 {
        transaction.rollback().await?;

        return JsonResponseBuilder::new(400)
            .set_error("UnknownIdsGiven")
            .set_message("some of the requested id were not found")
            .payload_response(unknown);
    }

    transaction.commit().await?;

    JsonResponseBuilder::new(200).response()
}