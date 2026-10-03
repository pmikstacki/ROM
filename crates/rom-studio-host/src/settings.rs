/// Presentation settings. Network endpoints and secrets remain host-approved configuration.
#[derive(Clone, rom::Resource)]
#[resource(name = "studio-settings")]
pub struct StudioSettings {
    /// An approved enabled provider, or the ordinary login selection screen.
    pub primary_provider: Option<String>,
}
