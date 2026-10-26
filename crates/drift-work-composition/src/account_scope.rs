//! The person's session on one site, as the controller's account key. The
//! key is minted per run and site; it keys taint and data flow and claims
//! nothing about who is signed in.
use drift_agent_controller::*;
use drift_agentic::*;

pub(crate) struct SessionAccount {
    pub(crate) account: AgentAccountId,
}
impl AgentWorkAccountSource for SessionAccount {
    fn sample(&self, context: ContextJoin) -> Result<AgentContextAccountBinding, AgentWorkFailure> {
        let observed_at =
            drift_engine::work_browser_monotonic_now().ok_or(AgentWorkFailure::Contract)?;
        Ok(AgentContextAccountBinding::new(
            AgentAccountAttestationId::generate(),
            context,
            AgentAccountScope::Authenticated(self.account),
            observed_at,
        ))
    }
}
