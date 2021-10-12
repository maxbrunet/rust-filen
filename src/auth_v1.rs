use crate::{settings::FilenSettings, utils};
use anyhow::*;
use secstr::SecUtf8;
use serde::{Deserialize, Serialize};
use serde_with::*;

const AUTH_INFO_PATH: &str = "/v1/auth/info";
const LOGIN_PATH: &str = "/v1/login";

/// Used for requests to [AUTH_INFO_PATH] endpoint.
#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthInfoRequestPayload {
    pub email: String,

    /// XXXXXX means no key
    #[serde(rename = "twoFactorKey")]
    pub two_factor_key: Option<SecUtf8>,
}

#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthInfoResponseData {
    pub email: String,

    /// Currently values of 1 & 2 can be encountered.
    #[serde(rename = "authVersion")]
    pub auth_version: u32,

    /// 256 alphanumeric characters or empty.
    pub salt: Option<String>,
}

/// Response for [AUTH_INFO_PATH] endpoint.
#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthInfoResponsePayload {
    pub status: bool,
    pub message: String,
    pub data: Option<AuthInfoResponseData>,
}

/// Used for requests to [LOGIN_PATH] endpoint.
#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LoginRequestPayload {
    pub email: String,

    pub password: SecUtf8,

    /// XXXXXX means no key
    #[serde(rename = "twoFactorKey")]
    pub two_factor_key: Option<SecUtf8>,

    /// Currently values of 1 & 2 can be encountered.
    #[serde(rename = "authVersion")]
    pub auth_version: u32,
}

#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LoginResponseData {
    #[serde(rename = "apiKey")]
    pub api_key: SecUtf8,

    /// This is plural, but seems to return just one key...
    #[serde(rename = "masterKeys")]
    pub master_keys: SecUtf8,

    #[serde(rename = "privateKey")]
    pub private_key: SecUtf8,
}

/// Response for [LOGIN_PATH] endpoint.
#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LoginResponsePayload {
    pub status: bool,
    pub message: String,
    pub data: Option<LoginResponseData>,
}

pub fn auth_info_request(
    payload: &AuthInfoRequestPayload,
    settings: &FilenSettings,
) -> Result<AuthInfoResponsePayload> {
    utils::query_filen_api(AUTH_INFO_PATH, payload, settings)
}

pub async fn auth_info_request_async(
    payload: &AuthInfoRequestPayload,
    settings: &FilenSettings,
) -> Result<AuthInfoResponsePayload> {
    utils::query_filen_api_async(AUTH_INFO_PATH, payload, settings).await
}

pub fn login_request(payload: &LoginRequestPayload, settings: &FilenSettings) -> Result<LoginResponsePayload> {
    utils::query_filen_api(LOGIN_PATH, payload, settings)
}

pub async fn login_request_async(
    payload: &LoginRequestPayload,
    settings: &FilenSettings,
) -> Result<LoginResponsePayload> {
    utils::query_filen_api_async(LOGIN_PATH, payload, settings).await
}

#[cfg(test)]
mod tests {
    use crate::{auth_v1::*, test_utils::*};
    use anyhow::Result;
    use closure::closure;
    use httpmock::Mock;
    use pretty_assertions::assert_eq;
    use tokio::task::spawn_blocking;

    #[tokio::test]
    async fn auth_info_request_and_async_should_work_with_v1() -> Result<()> {
        let (server, filen_settings) = init_server();
        let request_payload = AuthInfoRequestPayload {
            email: "test@email.com".to_owned(),
            two_factor_key: None,
        };
        let expected_response: AuthInfoResponsePayload =
            deserialize_from_file("tests/resources/responses/auth_info_v1.json");
        let mock: Mock = setup_json_mock(AUTH_INFO_PATH, &request_payload, &expected_response, &server);

        let response = spawn_blocking(
            closure!(clone request_payload, clone filen_settings, || { auth_info_request(&request_payload, &filen_settings) }),
        )
        .await??;
        mock.assert_hits(1);
        assert_eq!(response, expected_response);

        let async_response = auth_info_request_async(&request_payload, &filen_settings).await?;
        mock.assert_hits(2);
        assert_eq!(async_response, expected_response);
        Ok(())
    }

    #[tokio::test]
    async fn auth_info_request_and_async_should_work_with_v2() -> Result<()> {
        let (server, filen_settings) = init_server();
        let request_payload = AuthInfoRequestPayload {
            email: "test@email.com".to_owned(),
            two_factor_key: None,
        };
        let expected_response: AuthInfoResponsePayload =
            deserialize_from_file("tests/resources/responses/auth_info_v2.json");
        let mock: Mock = setup_json_mock(AUTH_INFO_PATH, &request_payload, &expected_response, &server);

        let response = spawn_blocking(
            closure!(clone request_payload, clone filen_settings, || auth_info_request(&request_payload, &filen_settings)),
        )
        .await??;
        mock.assert_hits(1);
        assert_eq!(response, expected_response);

        let async_response = auth_info_request_async(&request_payload, &filen_settings).await?;
        mock.assert_hits(2);
        assert_eq!(async_response, expected_response);
        Ok(())
    }

    #[tokio::test]
    async fn login_request_and_async_should_work_with_v1() -> Result<()> {
        let (server, filen_settings) = init_server();
        let request_payload = LoginRequestPayload {
            email: "test@email.com".to_owned(),
            password: SecUtf8::from("test"),
            two_factor_key: Some(SecUtf8::from("XXXXXX")),
            auth_version: 1,
        };
        let expected_response: LoginResponsePayload = deserialize_from_file("tests/resources/responses/login_v1.json");
        let mock: Mock = setup_json_mock(LOGIN_PATH, &request_payload, &expected_response, &server);

        let response = spawn_blocking(
            closure!(clone request_payload, clone filen_settings, || login_request(&request_payload, &filen_settings)),
        )
        .await??;
        mock.assert_hits(1);
        assert_eq!(response, expected_response);

        let async_response = login_request_async(&request_payload, &filen_settings).await?;
        mock.assert_hits(2);
        assert_eq!(async_response, expected_response);
        Ok(())
    }
}
