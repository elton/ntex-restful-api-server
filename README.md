# A Restful API server building with Ntex web framework

## Introduction

This is a simple Restful API server building with `Ntex` web framework. It provides a simple API to manage a list of users. The API server is built with `Ntex` web framework, and the data is stored in a PostgreSQL database.

## Technologies Stack

- [Rust](https://www.rust-lang.org/)
- [Ntex web framework](https://ntex.rs/)
- [PostgreSQL](https://www.postgresql.org/)
- [JWT](https://jwt.io/)

## Design

### Authentication Method

Use JWT(JSON Web Token) to authenticate users. The claims structure of the JWT follows:

```Rust
pub struct Claims {
    pub token_id: String, // token ID
    pub iss: String,      // issuer
    pub sub: String,      // subject
    pub iat: usize,       // issue date
    pub exp: usize,       // expire date
}
```

- After a successful login, two tokens will be generated, each with a unique ID generated by the ULID library.
- After the tokens are generated, they will be saved in the Redis server. Each token ID will be used as the key, with the corresponding user ID as the value, along with its expiration time. The expiration time for each token is set in a `.env` file. Finally, both tokens will be stored in the user's browser's `localStorage`.
- When a user requests an API from the server, the access token will be sent in the request header.
- The server will decode the access token to extract the token ID. Then, it will look up the Redis server to obtain the user ID associated with this token ID.
- To use the user ID that we obtained in the previous step, the server will search for the user information with this ID from the database. If the user is found, their API request will be executed. However, if the user is not found, the server will deny the request.
