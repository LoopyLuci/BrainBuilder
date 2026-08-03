use bot_server::luci_bridge::LuciBridge;
use bot_server::types::{ConversationId, IncomingMessage, LuciAction, UserId};

#[tokio::test]
async fn luci_bridge_slash_commands_and_actions() {
    let mut bridge = LuciBridge::new(ConversationId("c1".into()));

    let status = bridge
        .handle(IncomingMessage::Text {
            conversation: ConversationId("c1".into()),
            user: UserId("u1".into()),
            text: "/status".into(),
            platform: "telegram".into(),
            timestamp: 0,
            metadata: Default::default(),
        })
        .await;
    assert!(status.text.contains("Status:"));

    let remember = bridge
        .handle(IncomingMessage::Text {
            conversation: ConversationId("c1".into()),
            user: UserId("u1".into()),
            text: "/remember rust async".into(),
            platform: "telegram".into(),
            timestamp: 0,
            metadata: Default::default(),
        })
        .await;
    assert!(remember.text.contains("remember"));

    let plan = bridge
        .handle(IncomingMessage::Text {
            conversation: ConversationId("c1".into()),
            user: UserId("u1".into()),
            text: "/plan test | description".into(),
            platform: "telegram".into(),
            timestamp: 0,
            metadata: Default::default(),
        })
        .await;
    assert!(plan.text.contains("Created plan"));
    assert!(matches!(plan.actions.first(), Some(LuciAction::Plan { .. })));

    let improve = bridge
        .handle(IncomingMessage::Text {
            conversation: ConversationId("c1".into()),
            user: UserId("u1".into()),
            text: "/improve".into(),
            platform: "telegram".into(),
            timestamp: 0,
            metadata: Default::default(),
        })
        .await;
    assert!(improve.text.contains("self-improvement"));
    assert_eq!(improve.mood.as_deref(), Some("excited"));

    let call = bridge
        .handle(IncomingMessage::CallStart {
            conversation: ConversationId("c1".into()),
            user: UserId("u1".into()),
            platform: "telegram".into(),
            timestamp: 0,
        })
        .await;
    assert!(call.text.contains("Call started"));
}

#[tokio::test]
async fn unified_bot_starts_and_apply_actions() {
    let bot = bot_server::bot::UnifiedBot::start(
        std::net::SocketAddr::from(([127, 0, 0, 1], 0)),
        vec![],
    )
    .await
    .expect("start unified bot");

    bot.handle(IncomingMessage::Text {
        conversation: ConversationId("c1".into()),
        user: UserId("u1".into()),
        text: "/improve".into(),
        platform: "telegram".into(),
        timestamp: 0,
        metadata: Default::default(),
    })
    .await;

    let reply = bot_server::bot::UnifiedBot::start(
        std::net::SocketAddr::from(([127, 0, 0, 1], 0)),
        vec![],
    )
    .await
    .unwrap();
    reply.handle(IncomingMessage::Text {
        conversation: ConversationId("c1".into()),
        user: UserId("u1".into()),
        text: "/status".into(),
        platform: "telegram".into(),
        timestamp: 0,
        metadata: Default::default(),
    }).await;
}
