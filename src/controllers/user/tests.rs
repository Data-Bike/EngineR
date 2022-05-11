// #[cfg(test)]
// mod test {
//     use super::rocket;
//     use rocket::local::Client;
//     use rocket::http::Status;
//     use rand::rngs::StdRng;
//     use base64ct::{Base64, Encoding};
//     use rand::{RngCore, SeedableRng};
//     use rocket::local::blocking::Client;
//
//
//     pub fn randomString() -> String {
//         let mut enc_buf_token = [0u8; Self::token_length];
//         let mut enc_buf_base64 = [0u8; Self::token_length * 2];
//         let mut csprng = StdRng::from_entropy();
//         csprng.fill_bytes(&mut enc_buf_token);
//         let base64_hash = Base64::encode(&enc_buf_token, &mut enc_buf_base64).unwrap();
//         format!("{}", base64_hash)
//     }
//
//     #[test]
//     fn index() {
//         let client = Client::new(rocket()).expect("valid rocket instance");
//         let mut response = client.get("/").dispatch();
//         assert_eq!(response.status(), Status::Ok);
//         assert_eq!(response.body_string(), Some("Hello, world!".into()));
//     }
//
//     #[test]
//     fn testUser(){
//
//         let client1 = Client::new(rocket()).expect("valid rocket instance");
//         let login = randomString();
//         let password = randomString();
//         let cred = format!("{{\"login\":\"{}\", \"password\":\"{}\"}}",login,password);
//         let response1 = client1.post("/user/create")
//             .body(format!("{{\"login\":\"{}\", \"password\":\"{}\", firstname:\"Alexander\",lastname:\"Platonov\"}}",login, password))
//             .dispatch();
//         assert_eq!(response1.status(), Status::Ok);
//
//         let client2 = Client::new(rocket()).expect("valid rocket instance");
//         let response2 = client2.post("/user/login")
//             .body(cred.as_str())
//             .dispatch();
//         assert_eq!(response2.status(), Status::Ok);
//         assert_lg!(response2.cookies().len(), 1);
//
//
//
//         let response3 = client2.get(format!("/user/{}",login))
//             .dispatch();
//         assert_eq!(response3.status(), Status::Ok);
//         assert_eq!(response3.body(), format!("{{\"login\":\"{}\", firstname:\"Alexander\",lastname:\"Platonov\"}}",login));
//
//         let response4 = client2.post("/user/update")
//             .body(format!("{{\"login\":\"{}\", firstname:\"Alexander_2\",lastname:\"Platonov_2\"}}",login))
//             .dispatch();
//         assert_eq!(response4.status(), Status::Ok);
//
//         let response5 = client2.get(format!("/user/{}",login))
//             .dispatch();
//         assert_eq!(response5.status(), Status::Ok);
//         assert_eq!(response5.body(), format!("{{\"login\":\"{}\", firstname:\"Alexander_2\",lastname:\"Platonov_2\"}}",login));
//
//     }
// }