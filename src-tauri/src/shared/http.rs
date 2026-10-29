use std::sync::LazyLock;
use std::time::Duration;
use ureq::tls::{RootCerts, TlsConfig};
use ureq::Agent;

const TIMEOUT: Duration = Duration::from_secs(15);

pub fn client() -> &'static Agent {
    static CLIENT: LazyLock<Agent> = LazyLock::new(|| {
        let tls = TlsConfig::builder().root_certs(RootCerts::PlatformVerifier).build();
        Agent::config_builder()
            .timeout_global(Some(TIMEOUT))
            .user_agent("NC Workspaces")
            .http_status_as_error(false)
            .tls_config(tls)
            .build()
            .into()
    });
    &CLIENT
}
