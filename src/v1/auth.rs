use crate::{crypto, settings::FilenSettings, utils};
use anyhow::*;
use secstr::{SecUtf8, SecVec};
use serde::{Deserialize, Serialize};
use serde_with::*;

const AUTH_INFO_PATH: &str = "/v1/auth/info";
const LOGIN_PATH: &str = "/v1/login";

/// Used for requests to [AUTH_INFO_PATH] endpoint.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthInfoRequestPayload {
    /// Registered user email.
    pub email: SecUtf8,

    /// Registered user 2FA key, if present. XXXXXX means no 2FA key.
    #[serde(rename = "twoFactorKey")]
    pub two_factor_key: SecUtf8,
}

/// Response data for [AUTH_INFO_PATH] endpoint.
#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthInfoResponseData {
    /// Registered user email.
    pub email: SecUtf8,

    /// User-associated value which determines auth algorithm. Currently values of 1 & 2 can be encountered.
    /// 1 means [FilenPasswordWithMasterKey::from_user_password] should be used to generate Filen password for login;
    /// 2 means [FilenPasswordWithMasterKey::from_user_password_and_auth_info_salt] should be used instead.
    #[serde(rename = "authVersion")]
    pub auth_version: u32,

    /// 256 alphanumeric characters or empty.
    pub salt: Option<String>,
}

/// Response for [AUTH_INFO_PATH] endpoint.
#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AuthInfoResponsePayload {
    /// True when API call was successful; false otherwise.
    pub status: bool,

    /// Filen reason for success or failure.
    pub message: String,

    /// Actual API call data.
    pub data: Option<AuthInfoResponseData>,
}

/// Used for requests to [LOGIN_PATH] endpoint.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LoginRequestPayload {
    /// Registered user email.
    pub email: SecUtf8,

    /// Filen-processed password. Note that this is not a registered user password, but its hash.
    /// Use one of [FilenPasswordWithMasterKey]::from... methods to calculate it.
    pub password: SecUtf8,

    /// Registered user 2FA key, if present. XXXXXX means no 2FA key.
    #[serde(rename = "twoFactorKey")]
    pub two_factor_key: SecUtf8,

    /// Set this to a value you received from auth/info call and used to generate Filen password.
    #[serde(rename = "authVersion")]
    pub auth_version: u32,
}

/// Response data for [LOGIN_PATH] endpoint.
#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LoginResponseData {
    /// Filen API key.
    #[serde(rename = "apiKey")]
    pub api_key: SecUtf8,

    /// This string is a Filen metadata encrypted by the last master key and base64-encoded.
    /// It contains either a single master key string or multiple master keys strings delimited by '|'.
    /// Master key is in turn used to decrypt various metadata.
    /// Empty when no keys were set (currently before the first login).
    #[serde(rename = "masterKeys")]
    pub master_keys_metadata: Option<SecUtf8>,

    /// A user's RSA private key stored as Filen metadata encrypted by user's last master key, containing a base64-encoded key bytes.
    /// Private key is currently used for decrypting name and metadata of the shared download folders.
    /// Empty when no keys were set (currently before the first login).
    #[serde(rename = "privateKey")]
    pub private_key_metadata: Option<SecUtf8>,
}

impl LoginResponseData {
    /// Decrypts [LoginResponseData].master_keys_metadata field into a list of key strings,
    /// using specified user's last master key.
    pub fn decrypt_master_keys(&self, last_master_key: &SecUtf8) -> Result<Vec<SecUtf8>> {
        crypto::decrypt_master_keys_metadata(&self.master_keys_metadata, last_master_key)
    }

    /// Decrypts [LoginResponseData].private_key_metadata field into RSA key bytes,
    /// using specified user's last master key.
    pub fn decrypt_private_key(&self, last_master_key: &SecUtf8) -> Result<SecVec<u8>> {
        crypto::decrypt_private_key_metadata(&self.private_key_metadata, last_master_key)
    }
}

/// Response for [LOGIN_PATH] endpoint.
#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LoginResponsePayload {
    /// True when API call was successful; false otherwise.
    pub status: bool,

    /// Filen reason for success or failure.
    pub message: String,

    /// Actual API call data.
    pub data: Option<LoginResponseData>,
}

/// Calls [AUTH_INFO_PATH] endpoint. Used to get used auth version and Filen salt.
pub fn auth_info_request(
    payload: &AuthInfoRequestPayload,
    settings: &FilenSettings,
) -> Result<AuthInfoResponsePayload> {
    utils::query_filen_api(AUTH_INFO_PATH, payload, settings)
}

/// Calls [AUTH_INFO_PATH] endpoint asynchronously. Used to get used auth version and Filen salt.
pub async fn auth_info_request_async(
    payload: &AuthInfoRequestPayload,
    settings: &FilenSettings,
) -> Result<AuthInfoResponsePayload> {
    utils::query_filen_api_async(AUTH_INFO_PATH, payload, settings).await
}

/// Calls [LOGIN_PATH] endpoint. Used to get API key, master keys and private key.
pub fn login_request(payload: &LoginRequestPayload, settings: &FilenSettings) -> Result<LoginResponsePayload> {
    utils::query_filen_api(LOGIN_PATH, payload, settings)
}

/// Calls [LOGIN_PATH] endpoint asynchronously. Used to get API key, master keys and private key.
pub async fn login_request_async(
    payload: &LoginRequestPayload,
    settings: &FilenSettings,
) -> Result<LoginResponsePayload> {
    utils::query_filen_api_async(LOGIN_PATH, payload, settings).await
}

#[cfg(test)]
mod tests {
    use crate::{
        test_utils::{self, *},
        v1::auth::*,
    };
    use anyhow::Result;
    use closure::closure;
    use httpmock::Mock;
    use pretty_assertions::assert_eq;
    use tokio::task::spawn_blocking;

    #[test]
    fn login_response_data_should_decrypt_master_keys() {
        let m_key = SecUtf8::from("ed8d39b6c2d00ece398199a3e83988f1c4942b24");
        let master_keys_metadata_encrypted =
            SecUtf8::from("U2FsdGVkX1/P4QDMaiaanx8kpL7fY+v/f3dSzC9Ajl58gQg5bffqGUbOIzROwGQn8m5NAZa0tRnVya84aJnf1w==");
        let response_data = LoginResponseData {
            api_key: SecUtf8::from(""),
            master_keys_metadata: Some(master_keys_metadata_encrypted),
            private_key_metadata: Some(SecUtf8::from("")),
        };

        let decrypted_m_keys = response_data.decrypt_master_keys(&m_key).unwrap();

        assert_eq!(decrypted_m_keys.len(), 1);
        assert_eq!(decrypted_m_keys[0], m_key);
    }

    #[test]
    fn login_response_data_should_decrypt_private_key() {
        let m_key = SecUtf8::from("ed8d39b6c2d00ece398199a3e83988f1c4942b24");
        let expected_rsa_key_length = 2374;
        let private_key_file_contents = test_utils::read_project_file("tests/resources/filen_private_key.txt");
        let private_key_metadata_encrypted = String::from_utf8_lossy(&private_key_file_contents);
        let response_data = LoginResponseData {
            api_key: SecUtf8::from(""),
            master_keys_metadata: Some(SecUtf8::from("")),
            private_key_metadata: Some(SecUtf8::from(private_key_metadata_encrypted.clone())),
        };

        let decrypted_private_key = response_data.decrypt_private_key(&m_key).unwrap();

        assert_eq!(decrypted_private_key.unsecure().len(), expected_rsa_key_length);
    }

    #[tokio::test]
    async fn auth_info_request_and_async_should_work_with_v1() -> Result<()> {
        let (server, filen_settings) = init_server();
        let request_payload = AuthInfoRequestPayload {
            email: SecUtf8::from("test@email.com"),
            two_factor_key: SecUtf8::from("XXXXXX"),
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
            email: SecUtf8::from("test@email.com"),
            two_factor_key: SecUtf8::from("XXXXXX"),
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
            email: SecUtf8::from("test@email.com"),
            password: SecUtf8::from("test"),
            two_factor_key: SecUtf8::from("XXXXXX"),
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
