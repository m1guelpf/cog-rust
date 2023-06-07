use aide::openapi::{OpenApi, SchemaObject, StatusCode};
use axum::http::Method;
use schemars::schema::Schema;

pub fn replace_request_schema(
    api: &mut OpenApi,
    path: &str,
    (method, media_type): (Method, &str),
    json_schema: Schema,
) -> Option<()> {
    let paths = api.paths.as_mut()?;
    let item = paths.paths.get_mut(path)?.as_item_mut()?;
    let operation = match method {
        Method::GET => item.get.as_mut()?,
        Method::PUT => item.put.as_mut()?,
        Method::POST => item.post.as_mut()?,
        Method::HEAD => item.head.as_mut()?,
        Method::TRACE => item.trace.as_mut()?,
        Method::DELETE => item.delete.as_mut()?,
        Method::OPTIONS => item.options.as_mut()?,
        _ => return None,
    };

    let body = operation.request_body.as_mut()?.as_item_mut()?;

    body.content.get_mut(media_type)?.schema = Some(SchemaObject {
        json_schema,
        example: None,
        external_docs: None,
    });

    Some(())
}

pub fn replace_response_schema(
    api: &mut OpenApi,
    path: &str,
    (method, status_code, media_type): (Method, StatusCode, &str),
    json_schema: Schema,
) -> Option<()> {
    let paths = api.paths.as_mut()?;
    let item = paths.paths.get_mut(path)?.as_item_mut()?;
    let operation = match method {
        Method::GET => item.get.as_mut()?,
        Method::PUT => item.put.as_mut()?,
        Method::POST => item.post.as_mut()?,
        Method::HEAD => item.head.as_mut()?,
        Method::TRACE => item.trace.as_mut()?,
        Method::DELETE => item.delete.as_mut()?,
        Method::OPTIONS => item.options.as_mut()?,
        _ => return None,
    };

    let responses = operation.responses.as_mut()?;
    let response = responses
        .responses
        .get_mut(&status_code)
        .unwrap()
        .as_item_mut()?;

    response.content.get_mut(media_type)?.schema = Some(SchemaObject {
        json_schema,
        example: None,
        external_docs: None,
    });

    Some(())
}
