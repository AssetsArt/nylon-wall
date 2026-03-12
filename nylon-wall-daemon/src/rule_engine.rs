use nylon_wall_common::rule::FirewallRule;
use tracing::info;

pub struct RuleEngine {
    rules: Vec<FirewallRule>,
}

impl RuleEngine {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn add_rule(&mut self, rule: FirewallRule) {
        info!("Adding rule: {} (id={})", rule.name, rule.id);
        self.rules.push(rule);
        self.rules.sort_by_key(|r| r.priority);
    }

    pub fn remove_rule(&mut self, id: u32) -> Option<FirewallRule> {
        if let Some(pos) = self.rules.iter().position(|r| r.id == id) {
            Some(self.rules.remove(pos))
        } else {
            None
        }
    }

    pub fn get_rules(&self) -> &[FirewallRule] {
        &self.rules
    }

    /// Compile rules into eBPF maps (Linux only)
    pub fn sync_to_ebpf(&self) -> anyhow::Result<()> {
        // TODO: Write compiled rules to eBPF maps via aya
        info!("Syncing {} rules to eBPF maps", self.rules.len());
        Ok(())
    }
}
