use lightning::ln::{PaymentHash, PaymentPreimage};
use macaroon::{Macaroon, Verifier, MacaroonKey};
use rocket::{request, Request};
use hex;

use crate::lsat;

pub const LSAT_TYPE_FREE: &str = "FREE";
pub const LSAT_TYPE_PAYMENT_REQUIRED: &str = "PAYMENT REQUIRED";
pub const LSAT_TYPE_PAID: &str = "PAID";
pub const LSAT_TYPE_ERROR: &str = "ERROR";
pub const LSAT_HEADER: &str = "LSAT";
pub const LSAT_HEADER_NAME: &str = "Accept-Authenticate";
pub const LSAT_AUTHENTICATE_HEADER_NAME: &str = "WWW-Authenticate";
pub const LSAT_AUTHORIZATION_HEADER_NAME: &str = "Authorization";

#[derive(Clone)]
pub struct LsatInfo {
	pub	lsat_type: String,
	pub preimage: Option<PaymentPreimage>,
	pub payment_hash: Option<PaymentHash>,
	pub error: Option<String>,
    pub auth_header: Option<String>,
}

#[rocket::async_trait]
impl<'r> request::FromRequest<'r> for LsatInfo {
    type Error = &'static str;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        // Retrieve LsatInfo from the local cache
        let lsat_info = request.local_cache::<LsatInfo, _>(|| {
            LsatInfo {
                lsat_type: lsat::LSAT_TYPE_ERROR.to_string(),
                error: Some("No LSAT header present".to_string()),
                preimage: None,
                payment_hash: None,
                auth_header: None,
            }
        });

        request::Outcome::Success(lsat_info.clone())
    }
}

pub fn verify_lsat(
    mac: &Macaroon,
    caveats: Vec<String>,
    root_key: Vec<u8>,
    preimage: PaymentPreimage,
) -> Result<(), Box<dyn std::error::Error>> {
    // caveat verification
    let mac_caveats = mac.first_party_caveats();
    if caveats.len() > mac_caveats.len() {
        return Err("Error validating macaroon: Caveats don't match".into());
    }

    let mac_key = MacaroonKey::generate(&root_key);
    let mut verifier = Verifier::default();
    
    for caveat in caveats {
        verifier.satisfy_exact(caveat.into());
    }

    match verifier.verify(&mac, &mac_key, Default::default()) {
        Ok(_) => {
            let macaroon_id = mac.identifier().clone();
            let macaroon_id_hex = hex::encode(macaroon_id.0).replace("ff", "");
            let payment_hash: PaymentHash = PaymentHash::from(preimage);
            let payment_hash_hex = hex::encode(payment_hash.0);

            if macaroon_id_hex.contains(&payment_hash_hex) {
                Ok(())
            } else {
                Err(format!(
                    "Invalid PaymentHash {} for macaroon {}",
                    payment_hash_hex, macaroon_id_hex
                ).into())
            }
        },
        Err(error) => {
            Err(format!("Error validating macaroon: {:?}", error).into())
        }
    }
}
