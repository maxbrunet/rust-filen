pub mod auth;
pub mod dirs;
pub mod keys;

/// This macro generates a struct to parse Filen API response into.
///
/// Filen API uses same format for all its responses, successfull or not.
/// Status and message fields are always present, while data field can be returned on success,
/// when said success implies getting some data.
///
/// To use, pass generated struct name and contained data type:
/// ```
/// api_response_struct!(
///     /// Response for some endpoint.
///     SomeResponsePayload<Option<SomeResponseData>>
/// );
/// ```
macro_rules! api_response_struct {
    (
        $(#[$meta:meta])*
        $struct_name:ident<$response_data_type:ty>
    ) => {
        $(#[$meta])*
        #[skip_serializing_none]
        #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
        pub struct $struct_name {
            /// True when API call was successful; false otherwise.
            pub status: bool,

            /// Filen reason for success or failure.
            pub message: String,

            /// Resulting data.
            pub data: $response_data_type,
        }

        super::display_from_json!($struct_name);
    }
}
pub(crate) use api_response_struct;

/// This macro generates a simple [std::fmt::Display] implementation using Serde's json! on self.
macro_rules! display_from_json {
    (
        $target_data_type:ty
    ) => {
        impl std::fmt::Display for $target_data_type {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                match *self {
                    _ => write!(f, "{}", serde_json::json!(self)),
                }
            }
        }
    };
}
// TODO: Should this be a derive proc macro?
pub(crate) use display_from_json;
