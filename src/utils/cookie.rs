use cookie::{time, Cookie, CookieJar};

// store a cookie
pub fn store_cookie(name: &str, value: &str, max_age: i64) {
    // create a jar
    let mut jar = CookieJar::new();
    // create a cookie
    let cookie = Cookie::build((name.to_owned(), value.to_owned()))
        .path("/")
        .secure(true)
        .http_only(true)
        .max_age(time::Duration::minutes(max_age))
        .build();

    jar.add(cookie);
}
