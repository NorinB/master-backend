use anyhow::Context;
use anyhow::Result;
use log::info;
use wtransport::tls::Sha256DigestFmt;
use wtransport::Identity;

const CERT_FILE: &str = "certificates/cert.pem";
const KEY_FILE: &str = "certificates/key.pem";

pub async fn generate_certificate() -> Result<()> {
    info!("Generating self signed certificate for WebTransport");

    let identity = Identity::self_signed(["localhost", "127.0.0.1", "::1"])
        .context("cannot create self signed identity")?;

    info!("Storing certificate to file: '{CERT_FILE}'");

    identity
        .certificate_chain()
        .store_pemfile(CERT_FILE)
        .await
        .context("cannot store certificate chain")?;

    info!("Storing private key to file: '{KEY_FILE}'");

    identity
        .private_key()
        .store_secret_pemfile(KEY_FILE)
        .await
        .context("cannot store private key")?;

    info!(
        "Certificate serial: {}",
        identity.certificate_chain().as_slice()[0].serial()
    );

    info!(
        "Certificate hash: {}",
        identity.certificate_chain().as_slice()[0]
            .hash()
            .fmt(Sha256DigestFmt::BytesArray)
    );

    Ok(())
}
