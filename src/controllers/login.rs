use std::io::Read as io_Read;

use iron::prelude::*;
use iron_sessionstorage::traits::SessionRequestExt;
use hyper::Client;
use hyper::client::RequestBuilder;
use hyper::header::UserAgent;
use hyper::net::HttpsConnector;
use hyper_native_tls::NativeTlsClient;
use serde_json;
use serde_json::Value;

use url::{Url, form_urlencoded};
use persistent::Read;

use core::config::Config;
use core::http::*;
use core::utils::*;
use services::user::*;

pub fn render_login(req: &mut Request) -> IronResult<Response> {

    respond_view("login/index", &ResponseData::new(req))
}

pub fn login(req: &mut Request) -> IronResult<Response> {

    let session = req.session().get::<SessionObject>().unwrap();
    let params = get_request_body(req);
    let pool = get_mysql_pool(req);
    let username = &params.get("username").unwrap()[0];
    let password = &params.get("password").unwrap()[0];
    let user_id_wrapper = check_user_login(&pool, username, password);

    if user_id_wrapper.is_none() {  // 登录失败，该用户不存在！

        return redirect_to("http://localhost:3000/register");
    }

    let user_id = user_id_wrapper.unwrap();

    req.session().set(SessionObject {
        username: username.to_string()
    });

    redirect_to("http://localhost:3000")
}

pub fn github_auth_callback(req: &mut Request) -> IronResult<Response> {

    let params = get_request_query(req);
    let code = &params.get("code").unwrap()[0];
    let config = req.get::<Read<Config>>().unwrap().value();
    let github_config = config.get("github").unwrap().as_table().unwrap();
    let client_id = github_config.get("client_id").unwrap().as_str().unwrap();
    let client_secret = github_config.get("client_secret").unwrap().as_str().unwrap();

    let mut url = Url::parse("https://github.com/login/oauth/access_token").unwrap();
    url.query_pairs_mut()
        .append_pair("code", &code)
        .append_pair("client_id", &client_id)
        .append_pair("client_secret", &client_secret);

    let ssl = NativeTlsClient::new().unwrap();
    let connector = HttpsConnector::new(ssl);
    let client = Client::with_connector(connector);

    let mut body = String::new();
    client.get(url.as_str()).send().unwrap().read_to_string(&mut body).unwrap();

    let mut access_token = String::new();
    for (key, value) in form_urlencoded::parse(body.as_bytes()).into_owned() {
        if key == "access_token" {
            access_token = value;
        }
    }

    url = Url::parse("https://api.github.com/user").unwrap();
    url.query_pairs_mut()
        .append_pair("access_token", &access_token);

    body.clear();
    client.get(url.as_str())
            .header(UserAgent("runner1".to_string()))
            .send()
            .unwrap()
            .read_to_string(&mut body)
            .unwrap();

    let date: Value = serde_json::from_str(&*body).unwrap();

    println!("{:?}", date);

    redirect_to("http://localhost:3000")
}

