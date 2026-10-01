use super::*;
use shared::protocol::*;

mod transport;
pub(super) use transport::register;

impl EconomyOperation {
    fn is_mail(&self) -> bool {
        matches!(
            self,
            Self::MailList | Self::MailTakeMoney { .. } | Self::MailTakeAttachment { .. }
        )
    }

    fn is_merchant(&self) -> bool {
        matches!(self, Self::MerchantList | Self::MerchantBuy { .. })
    }
}

#[derive(Clone, Debug)]
pub enum EconomyOperation {
    Open,
    Search(AuctionSearchQuery),
    Browse(AuctionSearchQuery),
    Inventory,
    Owned,
    Bids,
    Create(CreateAuction),
    Bid(PlaceBid),
    Buyout(BuyoutAuction),
    Cancel(CancelAuction),
    MailList,
    MailTakeMoney { mail_id: u64 },
    MailTakeAttachment { mail_id: u64, slot: u8 },
    MerchantList,
    MerchantBuy { slot: u32, item_id: u32, count: u32 },
}

#[derive(Clone, Debug)]
pub struct EconomyAction {
    pub npc: u64,
    pub gossip_option: Option<u32>,
    pub operation: EconomyOperation,
}

#[derive(Debug, PartialEq, Eq)]
enum Phase {
    Idle,
    Interaction,
    Role,
    Opening,
    Request,
    Awaiting,
    Done,
}

#[derive(Resource)]
struct EconomyFlow {
    phase: Phase,
    started: std::time::Instant,
    open_sent: bool,
}

impl Default for EconomyFlow {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            started: std::time::Instant::now(),
            open_sent: false,
        }
    }
}

impl EconomyFlow {
    fn timeout(&self, elapsed: Duration) -> Option<String> {
        (self.phase != Phase::Done && elapsed > Duration::from_secs(10))
            .then(|| format!("Timeout after 10s waiting for {:?}", self.phase))
    }

    fn begin(&mut self, action: &EconomyAction) -> Option<InteractNpc> {
        if self.phase != Phase::Idle {
            return None;
        }
        self.phase = Phase::Interaction;
        self.started = std::time::Instant::now();
        Some(InteractNpc { npc: action.npc })
    }

    fn opened(
        &mut self,
        action: &EconomyAction,
        response: InteractionOpened,
    ) -> Result<Option<SelectGossipOption>, String> {
        if response.npc != action.npc || !matches!(self.phase, Phase::Interaction | Phase::Role) {
            return Ok(None);
        }
        match response.kind {
            InteractionKind::Gossip(menu) => {
                println!(
                    "[interaction] menu={}: {} options={:?}",
                    menu.menu_id, menu.text, menu.options
                );
                let option_id = action.gossip_option.ok_or_else(|| {
                    "NPC presented gossip; pass --gossip-option with an option ID printed above"
                        .to_string()
                })?;
                if !menu
                    .options
                    .iter()
                    .any(|option| option.option_id == option_id)
                {
                    return Err(format!("Gossip option {option_id} is not offered"));
                }
                self.phase = Phase::Role;
                Ok(Some(SelectGossipOption {
                    npc: action.npc,
                    option_id,
                }))
            }
            InteractionKind::Role(role) => {
                let expected = if action.operation.is_merchant() {
                    NpcRole::Vendor
                } else {
                    NpcRole::AuctionHouse
                };
                if role != expected {
                    return Err(format!("Expected {expected:?}, got {role:?}"));
                }
                self.phase = if action.operation.is_merchant() {
                    Phase::Request
                } else {
                    Phase::Opening
                };
                Ok(None)
            }
        }
    }

    fn house_opened(&mut self, response: AuctionHouseOpened) -> Result<(), String> {
        if !response.success {
            return Err(response
                .error
                .unwrap_or_else(|| "Auction house open rejected".into()));
        }
        if self.phase == Phase::Opening {
            self.phase = Phase::Request;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn action() -> EconomyAction {
        EconomyAction {
            npc: 77,
            gossip_option: Some(3),
            operation: EconomyOperation::Inventory,
        }
    }
    #[test]
    fn timeout_reports_pending_stage() {
        let mut flow = EconomyFlow::default();
        flow.begin(&action());
        assert!(
            flow.timeout(Duration::from_secs(11))
                .unwrap()
                .contains("Interaction")
        );
        assert_eq!(flow.timeout(Duration::from_secs(9)), None);
    }
    #[test]
    fn interaction_and_gossip_precede_auction_open_and_request() {
        let mut flow = EconomyFlow::default();
        assert_eq!(flow.begin(&action()), Some(InteractNpc { npc: 77 }));
        assert_eq!(flow.phase, Phase::Interaction);
        let choice = flow
            .opened(
                &action(),
                InteractionOpened {
                    npc: 77,
                    kind: InteractionKind::Gossip(GossipMenu {
                        menu_id: 1,
                        text: "Auction?".into(),
                        options: vec![GossipMenuOption {
                            option_id: 3,
                            icon: 0,
                            text: "Auction!".into(),
                        }],
                    }),
                },
            )
            .unwrap();
        assert_eq!(
            choice,
            Some(SelectGossipOption {
                npc: 77,
                option_id: 3
            })
        );
        assert_eq!(flow.phase, Phase::Role);
        flow.opened(
            &action(),
            InteractionOpened {
                npc: 77,
                kind: InteractionKind::Role(NpcRole::AuctionHouse),
            },
        )
        .unwrap();
        assert_eq!(flow.phase, Phase::Opening);
        flow.house_opened(AuctionHouseOpened {
            success: true,
            error: None,
        })
        .unwrap();
        assert_eq!(flow.phase, Phase::Request);
    }
    #[test]
    fn wrong_npc_cannot_advance_and_wrong_role_or_open_rejection_fail() {
        let mut flow = EconomyFlow::default();
        flow.begin(&action());
        flow.opened(
            &action(),
            InteractionOpened {
                npc: 78,
                kind: InteractionKind::Role(NpcRole::AuctionHouse),
            },
        )
        .unwrap();
        assert_eq!(flow.phase, Phase::Interaction);
        assert!(
            flow.opened(
                &action(),
                InteractionOpened {
                    npc: 77,
                    kind: InteractionKind::Role(NpcRole::Vendor)
                }
            )
            .is_err()
        );
        assert_eq!(
            flow.house_opened(AuctionHouseOpened {
                success: false,
                error: Some("rejected".into())
            }),
            Err("rejected".into())
        );
    }
}
