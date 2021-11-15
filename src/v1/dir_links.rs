use crate::{crypto, filen_settings::*, queries, utils, v1::*};
use secstr::SecUtf8;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use serde_with::skip_serializing_none;
use snafu::{ResultExt, Snafu};
use strum::{Display, EnumString};
use uuid::Uuid;

type Result<T, E = Error> = std::result::Result<T, E>;

pub(crate) static EMPTY_PASSWORD_VALUE: Lazy<String> = Lazy::new(|| PasswordState::Empty.to_string());
pub(crate) static SEC_EMPTY_PASSWORD_VALUE: Lazy<SecUtf8> = Lazy::new(|| SecUtf8::from(EMPTY_PASSWORD_VALUE.as_str()));
pub(crate) static EMPTY_PASSWORD_HASH: Lazy<String> = Lazy::new(|| crypto::hash_fn(&EMPTY_PASSWORD_VALUE));

const DIR_LINK_ADD_PATH: &str = "/v1/dir/link/add";
const DIR_LINK_EDIT_PATH: &str = "/v1/dir/link/edit";
const DIR_LINK_REMOVE_PATH: &str = "/v1/dir/link/remove";
const DIR_LINK_STATUS_PATH: &str = "/v1/dir/link/status";

#[allow(clippy::enum_variant_names)]
#[derive(Snafu, Debug)]
pub enum Error {
    #[snafu(display("{} query failed: {}", DIR_LINK_ADD_PATH, source))]
    DirLinkAddQueryFailed { source: queries::Error },

    #[snafu(display("{} query failed: {}", DIR_LINK_EDIT_PATH, source))]
    DirLinkEditQueryFailed { source: queries::Error },

    #[snafu(display("{} query failed: {}", DIR_LINK_REMOVE_PATH, source))]
    DirLinkRemoveQueryFailed { source: queries::Error },

    #[snafu(display("{} query failed: {}", DIR_LINK_STATUS_PATH, source))]
    DirLinkStatusQueryFailed { source: queries::Error },
}

/// State of the 'Enable download button' GUI toggle represented as a string.
/// It is the toggle you can see at the bottom of modal popup when creating or sharing an item.
#[derive(Clone, Debug, Deserialize, Display, EnumString, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
#[strum(ascii_case_insensitive, serialize_all = "lowercase")]
pub enum DownloadBtnState {
    /// 'Enable download button' checkbox is disabled.
    Disable,
    /// 'Enable download button' checkbox is enabled.
    Enable,
}

/// State of the 'Enable download button' GUI toggle represented as a 0|1 flag.
/// It is the toggle you can see at the bottom of modal popup when creating or sharing an item.
#[derive(Clone, Debug, Deserialize_repr, Display, EnumString, Eq, Hash, PartialEq, Serialize_repr)]
#[repr(u8)]
#[strum(ascii_case_insensitive, serialize_all = "lowercase")]
pub enum DownloadBtnStateByte {
    Disable = 0,
    Enable = 1,
}

/// Used for requests to [DIR_LINK_ADD_PATH] endpoint.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DirLinkAddRequestPayload {
    /// User-associated Filen API key.
    #[serde(rename = "apiKey")]
    pub api_key: SecUtf8,

    /// Filen sets this to "enable" by default.
    #[serde(rename = "downloadBtn")]
    pub download_btn: DownloadBtnState,

    /// Link expiration time in text form. Usually has value "never".
    pub expiration: Expire,

    /// Link key, encrypted.
    #[serde(rename = "key")]
    pub key_metadata: String,

    /// Link ID; hyphenated lowercased UUID V4.
    #[serde(rename = "linkUUID")]
    pub link_uuid: Uuid,

    /// Linked item metadata.
    pub metadata: String,

    /// ID of the parent of the linked item, hyphenated lowercased UUID V4 if non-base.
    /// Use "base" if the linked item is located in the root folder.
    pub parent: ParentOrBase,

    /// Filen always uses "empty" when adding links.
    pub password: PasswordState,

    /// Output of hash_fn for the link's password.
    #[serde(rename = "passwordHashed")]
    pub password_hashed: String,

    /// Determines whether a file or a folder is being linked.
    #[serde(rename = "type")]
    pub link_type: ItemKind,

    /// Linked item ID; hyphenated lowercased UUID V4.
    pub uuid: Uuid,
}
utils::display_from_json!(DirLinkAddRequestPayload);

impl DirLinkAddRequestPayload {
    pub fn new<S: Into<String>>(
        api_key: SecUtf8,
        linked_item_uuid: Uuid,
        linked_item_metadata: S,
        linked_item_parent_uuid: ParentOrBase,
        link_type: ItemKind,
        last_master_key: &SecUtf8,
    ) -> DirLinkAddRequestPayload {
        let link_key = utils::random_alphanumeric_string(32);
        let key_metadata = // Should never panic...
            crypto::encrypt_metadata_str(&link_key, last_master_key, METADATA_VERSION).unwrap();
        DirLinkAddRequestPayload {
            api_key,
            download_btn: DownloadBtnState::Enable,
            expiration: Expire::Never,
            key_metadata,
            link_uuid: Uuid::new_v4(),
            metadata: linked_item_metadata.into(),
            parent: linked_item_parent_uuid,
            password: PasswordState::Empty,
            password_hashed: EMPTY_PASSWORD_HASH.clone(),
            link_type,
            uuid: linked_item_uuid,
        }
    }
}

/// Used for requests to [DIR_LINK_EDIT_PATH] endpoint.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DirLinkEditRequestPayload {
    /// User-associated Filen API key.
    #[serde(rename = "apiKey")]
    pub api_key: SecUtf8,

    /// Filen sets this to "enable" by default. If user toggled off the 'Enable download button' checkbox,
    /// then this is set to "disable".
    #[serde(rename = "downloadBtn")]
    pub download_btn: DownloadBtnState,

    /// Link expiration time in text form. Usually has value "never".
    pub expiration: Expire,

    /// "empty" means no password protection, "notempty" means password is present.
    pub password: PasswordState,

    /// Hashed link's password, output of [crypto::derive_key_from_password_512] with 32 random bytes of salt;
    /// converted to a hex string.
    #[serde(rename = "passwordHashed")]
    pub password_hashed: String,

    /// Salt used to make hashed password.
    pub salt: String,

    /// Linked item ID; hyphenated lowercased UUID V4.
    pub uuid: Uuid,
}
utils::display_from_json!(DirLinkEditRequestPayload);

impl DirLinkEditRequestPayload {
    fn new<S: Into<String>>(
        api_key: SecUtf8,
        download_btn: DownloadBtnState,
        item_uuid: Uuid,
        expiration: Expire,
        link_plain_password: Option<&SecUtf8>,
    ) -> DirLinkEditRequestPayload {
        let (password_hashed, salt) = link_plain_password
            .map(|password| crypto::encrypt_to_link_password_and_salt(&password))
            .unwrap_or_else(|| crypto::encrypt_to_link_password_and_salt(&SEC_EMPTY_PASSWORD_VALUE));
        DirLinkEditRequestPayload {
            api_key,
            download_btn,
            expiration,
            password: link_plain_password
                .map(|_| PasswordState::NotEmpty)
                .unwrap_or(PasswordState::Empty),
            password_hashed,
            salt,
            uuid: item_uuid,
        }
    }
}

/// Used for requests to [DIR_LINK_REMOVE_PATH] endpoint.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DirLinkRemoveRequestPayload {
    /// User-associated Filen API key.
    #[serde(rename = "apiKey")]
    pub api_key: SecUtf8,

    /// Link ID; hyphenated lowercased UUID V4.
    pub uuid: Uuid,
}
utils::display_from_json!(DirLinkRemoveRequestPayload);

/// Used for requests to [DIR_LINK_STATUS_PATH] endpoint.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DirLinkStatusRequestPayload {
    /// User-associated Filen API key.
    #[serde(rename = "apiKey")]
    pub api_key: SecUtf8,

    /// ID of the item whose link should be checked; hyphenated lowercased UUID V4.
    pub uuid: Uuid,
}
utils::display_from_json!(DirLinkStatusRequestPayload);

/// Response data for [DIR_LINK_STATUS_PATH] endpoint.
#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct DirLinkStatusResponseData {
    /// True if link exists; false if link for the given item ID cannot be found.
    pub exists: bool,

    /// Found link ID; hyphenated lowercased UUID V4. None if no link was found.
    pub uuid: Option<Uuid>,

    /// Link key metadata. None if no link was found.
    pub key: Option<String>,

    /// Link expiration time, as Unix timestamp in seconds. None if no link was found.
    pub expiration: Option<u64>,

    /// Link expiration time in text form. None if no link was found.
    #[serde(rename = "expirationText")]
    pub expiration_text: Option<Expire>,

    /// None if no link was found.
    #[serde(rename = "downloadBtn")]
    pub download_btn: Option<DownloadBtnStateByte>,

    /// Link password hash in hex string form, or None if no password was set by user or if no link was found.
    pub password: Option<String>,
}
utils::display_from_json!(DirLinkStatusResponseData);

impl HasLinkKey for DirLinkStatusResponseData {
    fn link_key_metadata_ref(&self) -> Option<&str> {
        self.key.as_deref()
    }
}

response_payload!(
    /// Response for [DIR_LINK_STATUS_PATH] endpoint.
    DirLinkStatusResponsePayload<DirLinkStatusResponseData>
);

/// Calls [DIR_LINK_ADD_PATH] endpoint. Used to add a folder or a file to a folder link.
///
/// Filen always creates a link without password first, and optionally sets password later using [dir_link_edit_request].
pub fn dir_link_add_request(
    payload: &DirLinkAddRequestPayload,
    filen_settings: &FilenSettings,
) -> Result<PlainResponsePayload> {
    queries::query_filen_api(DIR_LINK_ADD_PATH, payload, filen_settings).context(DirLinkAddQueryFailed {})
}

/// Calls [DIR_LINK_ADD_PATH] endpoint asynchronously. Used to add a folder or a file to a folder link.
///
/// Filen always creates a link without password first, and optionally sets password later using [dir_link_edit_request].
#[cfg(feature = "async")]
pub async fn dir_link_add_request_async(
    payload: &DirLinkAddRequestPayload,
    filen_settings: &FilenSettings,
) -> Result<PlainResponsePayload> {
    queries::query_filen_api_async(DIR_LINK_ADD_PATH, payload, filen_settings)
        .await
        .context(DirLinkAddQueryFailed {})
}

/// Calls [DIR_LINK_EDIT_PATH] endpoint. Used to edit given folder link.
///
/// Filen always creates a link without password first, and optionally sets password later using this query.
pub fn dir_link_edit_request(
    payload: &DirLinkEditRequestPayload,
    filen_settings: &FilenSettings,
) -> Result<PlainResponsePayload> {
    queries::query_filen_api(DIR_LINK_EDIT_PATH, payload, filen_settings).context(DirLinkEditQueryFailed {})
}

/// Calls [DIR_LINK_EDIT_PATH] endpoint asynchronously. Used to edit given folder link.
///
/// Filen always creates a link without password first, and optionally sets password later using this query.
#[cfg(feature = "async")]
pub async fn dir_link_edit_request_async(
    payload: &DirLinkEditRequestPayload,
    filen_settings: &FilenSettings,
) -> Result<PlainResponsePayload> {
    queries::query_filen_api_async(DIR_LINK_EDIT_PATH, payload, filen_settings)
        .await
        .context(DirLinkEditQueryFailed {})
}

/// Calls [DIR_LINK_REMOVE_PATH] endpoint. Used to remove given folder link.
pub fn dir_link_remove_request(
    payload: &DirLinkRemoveRequestPayload,
    filen_settings: &FilenSettings,
) -> Result<PlainResponsePayload> {
    queries::query_filen_api(DIR_LINK_REMOVE_PATH, payload, filen_settings).context(DirLinkRemoveQueryFailed {})
}

/// Calls [DIR_LINK_REMOVE_PATH] endpoint asynchronously. Used to remove given folder link.
#[cfg(feature = "async")]
pub async fn dir_link_remove_request_async(
    payload: &DirLinkRemoveRequestPayload,
    filen_settings: &FilenSettings,
) -> Result<PlainResponsePayload> {
    queries::query_filen_api_async(DIR_LINK_REMOVE_PATH, payload, filen_settings)
        .await
        .context(DirLinkRemoveQueryFailed {})
}

/// Calls [DIR_LINK_STATUS_PATH] endpoint. Used to check folder link status.
pub fn dir_link_status_request(
    payload: &DirLinkStatusRequestPayload,
    filen_settings: &FilenSettings,
) -> Result<DirLinkStatusResponsePayload> {
    queries::query_filen_api(DIR_LINK_STATUS_PATH, payload, filen_settings).context(DirLinkStatusQueryFailed {})
}

/// Calls [DIR_LINK_STATUS_PATH] endpoint asynchronously. Used to check folder link status.
#[cfg(feature = "async")]
pub async fn dir_link_status_request_async(
    payload: &DirLinkStatusRequestPayload,
    filen_settings: &FilenSettings,
) -> Result<DirLinkStatusResponsePayload> {
    queries::query_filen_api_async(DIR_LINK_STATUS_PATH, payload, filen_settings)
        .await
        .context(DirLinkStatusQueryFailed {})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use once_cell::sync::Lazy;
    use secstr::SecUtf8;

    static API_KEY: Lazy<SecUtf8> =
        Lazy::new(|| SecUtf8::from("bYZmrwdVEbHJSqeA1RfnPtKiBcXzUpRdKGRkjw9m1o1eqSGP1s6DM11CDnklpFq6"));

    #[test]
    fn dir_link_status_request_should_have_proper_contract_for_no_link() {
        let request_payload = DirLinkStatusRequestPayload {
            api_key: API_KEY.clone(),
            uuid: Uuid::nil(),
        };
        validate_contract(
            DIR_LINK_STATUS_PATH,
            request_payload,
            "tests/resources/responses/dir_link_status_no_link.json",
            |request_payload, filen_settings| dir_link_status_request(&request_payload, &filen_settings),
        );
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn dir_link_status_request_async_should_have_proper_contract_for_no_link() {
        let request_payload = DirLinkStatusRequestPayload {
            api_key: API_KEY.clone(),
            uuid: Uuid::nil(),
        };
        validate_contract_async(
            DIR_LINK_STATUS_PATH,
            request_payload,
            "tests/resources/responses/dir_link_status_no_link.json",
            |request_payload, filen_settings| async move {
                dir_link_status_request_async(&request_payload, &filen_settings).await
            },
        )
        .await;
    }

    #[test]
    fn dir_link_status_request_should_have_proper_contract_for_link_without_password() {
        let request_payload = DirLinkStatusRequestPayload {
            api_key: API_KEY.clone(),
            uuid: Uuid::nil(),
        };
        validate_contract(
            DIR_LINK_STATUS_PATH,
            request_payload,
            "tests/resources/responses/dir_link_status_no_password.json",
            |request_payload, filen_settings| dir_link_status_request(&request_payload, &filen_settings),
        );
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn dir_link_status_request_async_should_have_proper_contract_for_link_without_password() {
        let request_payload = DirLinkStatusRequestPayload {
            api_key: API_KEY.clone(),
            uuid: Uuid::nil(),
        };
        validate_contract_async(
            DIR_LINK_STATUS_PATH,
            request_payload,
            "tests/resources/responses/dir_link_status_no_password.json",
            |request_payload, filen_settings| async move {
                dir_link_status_request_async(&request_payload, &filen_settings).await
            },
        )
        .await;
    }
}
