use anyhow::bail;
use once_cell::sync::Lazy;
use openssl::pkey::{PKey, Private, Public};
use openssl::sign::{Signer, Verifier};
use serde_json::Value;
use std::io::Read;

use crate::der::oid::ObjectIdentifier;
use crate::der::{DerBuilder, DerReader, DerType};
use crate::error::JoseError;
use crate::jws::{JwsAlgorithm, JwsSigner, JwsVerifier};
use crate::jwk::Jwk;
use crate::util::parse_pem;

static OID_ED25519: Lazy<ObjectIdentifier> =
    Lazy::new(|| ObjectIdentifier::from_slice(&[1, 3, 101, 112]));

static OID_ED448: Lazy<ObjectIdentifier> =
    Lazy::new(|| ObjectIdentifier::from_slice(&[1, 3, 101, 113]));

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum EddsaJwsAlgorithm {
    /// EdDSA signature algorithms
    EDDSA,
}

impl EddsaJwsAlgorithm {
    /// Return a signer from a private key of common or traditinal PEM format.
    ///
    /// Common PEM format is a DER and base64 encoded PKCS#8 PrivateKeyInfo
    /// that surrounded by "-----BEGIN/END PRIVATE KEY----".
    ///
    /// Traditional PEM format is a DER and base64 encoded PKCS#8 PrivateKeyInfo
    /// that surrounded by "-----BEGIN/END ED25519/ED448 PRIVATE KEY----".
    ///
    /// # Arguments
    /// * `input` - A private key of common or traditinal PEM format.
    pub fn signer_from_pem(&self, input: impl AsRef<[u8]>) -> Result<EddsaJwsSigner, JoseError> {
        (|| -> anyhow::Result<EddsaJwsSigner> {
            let (alg, data) = parse_pem(input.as_ref())?;
            let pkey = match alg.as_str() {
                "PRIVATE KEY" => {
                    if let Some(_) = self.detect_pkcs8(&data, false)? {
                        PKey::private_key_from_der(&data)?
                    } else {
                        bail!("The EdDSA private key must be wrapped by PKCS#8 format.");
                    }
                }
                "ED25519 PRIVATE KEY" => {
                    if let Some(oid) = self.detect_pkcs8(&data, false)? {
                        if oid == *OID_ED25519 {
                            PKey::private_key_from_der(&data)?
                        } else {
                            bail!("The EdDSA curve is mismatched: {}", oid);
                        }
                    } else {
                        bail!("The EdDSA private key must be wrapped by PKCS#8 format.");
                    }
                }
                "ED448 PRIVATE KEY" => {
                    if let Some(oid) = self.detect_pkcs8(&data, false)? {
                        if oid == *OID_ED448 {
                            PKey::private_key_from_der(&data)?
                        } else {
                            bail!("The EdDSA curve is mismatched: {}", oid);
                        }
                    } else {
                        bail!("The EdDSA private key must be wrapped by PKCS#8 format.");
                    }
                }
                alg => bail!("Inappropriate algorithm: {}", alg),
            };

            Ok(EddsaJwsSigner {
                algorithm: self.clone(),
                private_key: pkey,
                key_id: None,
            })
        })()
        .map_err(|err| JoseError::InvalidKeyFormat(err))
    }

    /// Return a signer from a private key that is a DER encoded PKCS#8 PrivateKeyInfo.
    ///
    /// # Arguments
    /// * `input` - A private key that is a DER encoded PKCS#8 PrivateKeyInfo.
    pub fn signer_from_der(&self, input: impl AsRef<[u8]>) -> Result<EddsaJwsSigner, JoseError> {
        (|| -> anyhow::Result<EddsaJwsSigner> {
            let pkey = if let Some(_) = self.detect_pkcs8(input.as_ref(), false)? {
                PKey::private_key_from_der(input.as_ref())?
            } else {
                bail!("The EdDSA private key must be wrapped by PKCS#8 format.");
            };

            Ok(EddsaJwsSigner {
                algorithm: self.clone(),
                private_key: pkey,
                key_id: None,
            })
        })()
        .map_err(|err| JoseError::InvalidKeyFormat(err))
    }

    /// Return a verifier from a key of common or traditional PEM format.
    ///
    /// Common PEM format is a DER and base64 encoded SubjectPublicKeyInfo
    /// that surrounded by "-----BEGIN/END PUBLIC KEY----".
    ///
    /// Traditional PEM format is a DER and base64 SubjectPublicKeyInfo
    /// that surrounded by "-----BEGIN/END ED25519/ED448 PUBLIC KEY----".
    ///
    /// # Arguments
    /// * `input` - A key of common or traditional PEM format.
    pub fn verifier_from_pem(
        &self,
        input: impl AsRef<[u8]>,
    ) -> Result<EddsaJwsVerifier, JoseError> {
        (|| -> anyhow::Result<EddsaJwsVerifier> {
            let (alg, data) = parse_pem(input.as_ref())?;
            let pkey = match alg.as_str() {
                "PUBLIC KEY" => {
                    if let Some(_) = self.detect_pkcs8(&data, true)? {
                        PKey::public_key_from_der(&data)?
                    } else {
                        bail!("The EdDSA public key must be wrapped by PKCS#8 format.");
                    }
                }
                "ED25519 PUBLIC KEY" => {
                    if let Some(oid) = self.detect_pkcs8(&data, true)? {
                        if oid == *OID_ED25519 {
                            PKey::public_key_from_der(&data)?
                        } else {
                            bail!("The EdDSA curve is mismatched: {}", oid);
                        }
                    } else {
                        bail!("The EdDSA public key must be wrapped by PKCS#8 format.");
                    }
                }
                "ED448 PUBLIC KEY" => {
                    if let Some(oid) = self.detect_pkcs8(&data, true)? {
                        if oid == *OID_ED448 {
                            PKey::public_key_from_der(&data)?
                        } else {
                            bail!("The EdDSA curve is mismatched: {}", oid);
                        }
                    } else {
                        bail!("The EdDSA public key must be wrapped by PKCS#8 format.");
                    }
                }
                alg => bail!("Unacceptable algorithm: {}", alg),
            };

            Ok(EddsaJwsVerifier {
                algorithm: self.clone(),
                public_key: pkey,
                key_id: None,
            })
        })()
        .map_err(|err| JoseError::InvalidKeyFormat(err))
    }

    /// Return a verifier from a public key that is a DER encoded SubjectPublicKeyInfo.
    ///
    /// # Arguments
    /// * `input` - A public key that is a DER encoded SubjectPublicKeyInfo.
    pub fn verifier_from_der(
        &self,
        input: impl AsRef<[u8]>,
    ) -> Result<EddsaJwsVerifier, JoseError> {
        (|| -> anyhow::Result<EddsaJwsVerifier> {
            let pkey = if let Some(_) = self.detect_pkcs8(input.as_ref(), true)? {
                PKey::public_key_from_der(input.as_ref())?
            } else {
                bail!("The EdDSA public key must be wrapped by PKCS#8 format.");
            };

            Ok(EddsaJwsVerifier {
                algorithm: self.clone(),
                public_key: pkey,
                key_id: None,
            })
        })()
        .map_err(|err| JoseError::InvalidKeyFormat(err))
    }

    fn detect_pkcs8(
        &self,
        input: &[u8],
        is_public: bool,
    ) -> anyhow::Result<Option<ObjectIdentifier>> {
        let oid;
        let mut reader = DerReader::from_reader(input);

        match reader.next() {
            Ok(Some(DerType::Sequence)) => {}
            _ => return Ok(None),
        }

        {
            if !is_public {
                // Version
                match reader.next() {
                    Ok(Some(DerType::Integer)) => match reader.to_u8() {
                        Ok(val) => {
                            if val != 0 {
                                bail!("Unrecognized version: {}", val);
                            }
                        }
                        _ => return Ok(None),
                    },
                    _ => return Ok(None),
                }
            }

            match reader.next() {
                Ok(Some(DerType::Sequence)) => {}
                _ => return Ok(None),
            }

            {
                match reader.next() {
                    Ok(Some(DerType::ObjectIdentifier)) => match reader.to_object_identifier() {
                        Ok(val) => {
                            if val != *OID_ED25519 && val != *OID_ED448 {
                                bail!("Unsupported curve OID: {}", val);
                            }
                            oid = val;
                        }
                        _ => return Ok(None),
                    },
                    _ => return Ok(None),
                }
            }
        }

        Ok(Some(oid))
    }

    fn to_pkcs8(&self, input: &[u8], is_public: bool, crv: &ObjectIdentifier) -> Vec<u8> {
        let mut builder = DerBuilder::new();
        builder.begin(DerType::Sequence);
        {
            if !is_public {
                builder.append_integer_from_u8(0);
            }

            builder.begin(DerType::Sequence);
            {
                builder.append_object_identifier(crv);
            }
            builder.end();

            if is_public {
                builder.append_bit_string_from_slice(input, 0);
            } else {
                builder.append_octed_string_from_slice(input);
            }
        }
        builder.end();

        builder.build()
    }
}

impl JwsAlgorithm for EddsaJwsAlgorithm {
    fn name(&self) -> &str {
        "EdDSA"
    }

    fn key_type(&self) -> &str {
        "OKP"
    }

    fn signer_from_jwk(&self, jwk: &Jwk) -> Result<Box<dyn JwsSigner>, JoseError> {
        (|| -> anyhow::Result<Box<dyn JwsSigner>> {
            match jwk.key_type() {
                val if val == self.key_type() => {}
                val => bail!("A parameter kty must be {}: {}", self.key_type(), val),
            }
            match jwk.key_use() {
                Some(val) if val == "sig" => {},
                None => {}
                Some(val) => bail!("A parameter use must be sig: {}", val),
            }
            match jwk.key_operations() {
                Some(vals) if vals.iter().any(|e| e == "sign") => {}
                None => {}
                _ => bail!("A parameter key_ops must contains sign."),
            }
            match jwk.algorithm() {
                Some(val) if val == self.name() => {}
                None => {}
                Some(val) => bail!("A parameter alg must be {} but {}", self.name(), val),
            }
            let key_id = jwk.key_id();

            let curve = match jwk.parameter("crv") {
                Some(Value::String(val)) if val == "Ed25519" => &OID_ED25519,
                Some(Value::String(val)) if val == "Ed448" => &OID_ED448,
                Some(Value::String(val)) => bail!("A parameter crv must is invalid: {}", val),
                Some(_) => bail!("A parameter crv must be a string."),
                None => bail!("A parameter crv is required."),
            };
            let d = match jwk.parameter("d") {
                Some(Value::String(val)) => base64::decode_config(
                    val,
                    base64::URL_SAFE_NO_PAD,
                )?,
                Some(_) => bail!("A parameter d must be a string."),
                None => bail!("A parameter d is required."),
            };

            let mut builder = DerBuilder::new();
            builder.append_octed_string_from_slice(&d);

            let pkcs8 = self.to_pkcs8(&builder.build(), false, curve);
            let pkey = PKey::private_key_from_der(&pkcs8)?;

            Ok(Box::new(EddsaJwsSigner {
                algorithm: self.clone(),
                private_key: pkey,
                key_id: key_id.map(|val| val.to_string()),
            }))
        })()
        .map_err(|err| JoseError::InvalidKeyFormat(err))
    }

    fn verifier_from_jwk(&self, jwk: &Jwk) -> Result<Box<dyn JwsVerifier>, JoseError> {
        (|| -> anyhow::Result<Box<dyn JwsVerifier>> {
            match jwk.key_type() {
                val if val == self.key_type() => {}
                val => bail!("A parameter kty must be {}: {}", self.key_type(), val),
            }
            match jwk.key_use() {
                Some(val) if val == "sig" => {},
                None => {}
                Some(val) => bail!("A parameter use must be sig: {}", val),
            }
            match jwk.key_operations() {
                Some(vals) if vals.iter().any(|e| e == "verify") => {}
                None => {}
                _ => bail!("A parameter key_ops must contains verify."),
            }
            match jwk.algorithm() {
                Some(val) if val == self.name() => {}
                None => {}
                Some(val) => bail!("A parameter alg must be {} but {}", self.name(), val),
            }
            let key_id = jwk.key_id();

            let curve = match jwk.parameter("crv") {
                Some(Value::String(val)) if val == "Ed25519" => &OID_ED25519,
                Some(Value::String(val)) if val == "Ed448" => &OID_ED448,
                Some(Value::String(val)) => bail!("A parameter crv must is invalid: {}", val),
                Some(_) => bail!("A parameter crv must be a string."),
                None => bail!("A parameter crv is required."),
            };
            let x = match jwk.parameter("x") {
                Some(Value::String(val)) => base64::decode_config(
                    val,
                    base64::URL_SAFE_NO_PAD,
                )?,
                Some(_) => bail!("A parameter x must be a string."),
                None => bail!("A parameter x is required."),
            };

            let pkcs8 = self.to_pkcs8(&x, true, curve);
            let pkey = PKey::public_key_from_der(&pkcs8)?;

            Ok(Box::new(EddsaJwsVerifier {
                algorithm: self.clone(),
                public_key: pkey,
                key_id: key_id.map(|val| val.to_string()),
            }))
        })()
        .map_err(|err| JoseError::InvalidKeyFormat(err))
    }
}

pub struct EddsaJwsSigner {
    algorithm: EddsaJwsAlgorithm,
    private_key: PKey<Private>,
    key_id: Option<String>,
}

impl JwsSigner for EddsaJwsSigner {
    fn algorithm(&self) -> &dyn JwsAlgorithm {
        &self.algorithm
    }

    fn key_id(&self) -> Option<&str> {
        match &self.key_id {
            Some(val) => Some(val.as_ref()),
            None => None,
        }
    }

    fn set_key_id(&mut self, key_id: &str) {
        self.key_id = Some(key_id.to_string());
    }

    fn remove_key_id(&mut self) {
        self.key_id = None;
    }

    fn sign(&self, message: &mut dyn Read) -> Result<Vec<u8>, JoseError> {
        (|| -> anyhow::Result<Vec<u8>> {
            let mut signer = Signer::new_without_digest(&self.private_key)?;
            let mut signature = vec![0; signer.len()?];

            let mut buf = Vec::new();
            let _ = message.read_to_end(&mut buf);

            signer.sign_oneshot(&mut signature, &buf)?;
            Ok(signature)
        })()
        .map_err(|err| JoseError::InvalidSignature(err))
    }
}

pub struct EddsaJwsVerifier {
    algorithm: EddsaJwsAlgorithm,
    public_key: PKey<Public>,
    key_id: Option<String>,
}

impl JwsVerifier for EddsaJwsVerifier {
    fn algorithm(&self) -> &dyn JwsAlgorithm {
        &self.algorithm
    }

    fn key_id(&self) -> Option<&str> {
        match &self.key_id {
            Some(val) => Some(val.as_ref()),
            None => None,
        }
    }

    fn set_key_id(&mut self, key_id: &str) {
        self.key_id = Some(key_id.to_string());
    }

    fn unset_key_id(&mut self) {
        self.key_id = None;
    }

    fn verify(&self, message: &mut dyn Read, signature: &[u8]) -> Result<(), JoseError> {
        (|| -> anyhow::Result<()> {
            let mut verifier = Verifier::new_without_digest(&self.public_key)?;

            let mut buf = Vec::new();
            let _ = message.read_to_end(&mut buf);

            verifier.verify_oneshot(signature, &buf)?;
            Ok(())
        })()
        .map_err(|err| JoseError::InvalidSignature(err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use anyhow::Result;
    use std::fs::File;
    use std::io::{Cursor, Read};
    use std::path::PathBuf;

    #[test]
    fn sign_and_verify_eddsa_jwt() -> Result<()> {
        let input = b"abcde12345";

        let alg = EddsaJwsAlgorithm::EDDSA;

        let private_key = load_file("jwk/OKP_Ed25519_private.jwk")?;
        let public_key = load_file("jwk/OKP_Ed25519_private.jwk")?;

        let signer = alg.signer_from_jwk(&Jwk::from_slice(&private_key)?)?;
        let signature = signer.sign(&mut Cursor::new(input))?;

        let verifier = alg.verifier_from_jwk(&Jwk::from_slice(&public_key)?)?;
        verifier.verify(&mut Cursor::new(input), &signature)?;

        Ok(())
    }

    #[test]
    fn sign_and_verify_eddsa_pkcs8_pem() -> Result<()> {
        let input = b"abcde12345";

        let alg = EddsaJwsAlgorithm::EDDSA;

        for crv in &["ED25519", "ED448"] {
            let private_key = load_file(&format!("pem/{}_pkcs8_private.pem", crv))?;
            let public_key = load_file(&format!("pem/{}_pkcs8_public.pem", crv))?;

            let signer = alg.signer_from_pem(&private_key)?;
            let signature = signer.sign(&mut Cursor::new(input))?;

            let verifier = alg.verifier_from_pem(&public_key)?;
            verifier.verify(&mut Cursor::new(input), &signature)?;
        }

        Ok(())
    }

    #[test]
    fn sign_and_verify_eddsa_pkcs8_der() -> Result<()> {
        let input = b"abcde12345";

        let alg = EddsaJwsAlgorithm::EDDSA;

        for crv in &["ED25519", "ED448"] {
            let private_key = load_file(&format!("der/{}_pkcs8_private.der", crv))?;
            let public_key = load_file(&format!("der/{}_pkcs8_public.der", crv))?;

            let signer = alg.signer_from_der(&private_key)?;
            let signature = signer.sign(&mut Cursor::new(input))?;

            let verifier = alg.verifier_from_der(&public_key)?;
            verifier.verify(&mut Cursor::new(input), &signature)?;
        }

        Ok(())
    }

    fn load_file(path: &str) -> Result<Vec<u8>> {
        let mut pb = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        pb.push("data");
        pb.push(path);

        let mut file = File::open(&pb)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        Ok(data)
    }
}
