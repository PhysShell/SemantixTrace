//! Property: for stateless oracles, evaluation order does not change results
//! (oracle commutativity).

use chrono::DateTime;
use proptest::prelude::*;
use trace_core::{CommandId, EventSeq, Outcome, ScreenId, Session, SessionId};
use trace_oracle::{
    NoErrorModalAfterCommand, NoUnhandledException, OracleEngine, ScreenMustNavigateForward,
};
use trace_schema::v1::TraceEventKind;
use trace_schema::Current;
use uuid::Uuid;

fn arb_event_kind() -> impl Strategy<Value = TraceEventKind> {
    prop_oneof![
        Just(TraceEventKind::ScreenOpened {
            screen_id: ScreenId::new("Editor"),
            params: serde_json::json!({}),
        }),
        Just(TraceEventKind::CommandExecuted {
            command_id: CommandId::new("Cmd.Save"),
            args: serde_json::json!({}),
            duration_ms: 1,
            outcome: Outcome::Success,
        }),
        Just(TraceEventKind::ExceptionThrown {
            exception_type: "NullRef".into(),
            message: "boom".into(),
            stack: None,
        }),
        Just(TraceEventKind::NavigationOccurred {
            from: ScreenId::new("Editor"),
            to: ScreenId::new("List"),
        }),
    ]
}

fn make_session(kinds: Vec<TraceEventKind>) -> Option<Session<Current>> {
    let sid = SessionId::new(Uuid::from_u128(42));
    let events: Vec<Current> = kinds
        .into_iter()
        .enumerate()
        .map(|(i, kind)| Current {
            seq: EventSeq::new(i as u64),
            session_id: sid,
            ts: DateTime::from_timestamp(0, 0).unwrap(),
            correlation_id: None,
            kind,
        })
        .collect();
    if events.is_empty() {
        return None;
    }
    Session::new(sid, events).ok()
}

proptest! {
    #[test]
    fn oracle_commutativity(kinds in proptest::collection::vec(arb_event_kind(), 1..10)) {
        let Some(session) = make_session(kinds) else { return Ok(()); };

        let mut engine1 = OracleEngine::new();
        engine1.add_rule(Box::new(NoUnhandledException));
        engine1.add_rule(Box::new(NoErrorModalAfterCommand));
        engine1.add_rule(Box::new(ScreenMustNavigateForward));

        let mut engine2 = OracleEngine::new();
        engine2.add_rule(Box::new(ScreenMustNavigateForward));
        engine2.add_rule(Box::new(NoErrorModalAfterCommand));
        engine2.add_rule(Box::new(NoUnhandledException));

        let r1 = engine1.run(&session);
        let r2 = engine2.run(&session);

        // Sort by rule name for comparison regardless of evaluation order.
        let mut results1 = r1.results;
        results1.sort_by_key(|r| r.rule.clone());
        let mut results2 = r2.results;
        results2.sort_by_key(|r| r.rule.clone());

        prop_assert_eq!(
            results1.iter().map(|r| (r.rule.as_str(), r.passed)).collect::<Vec<_>>(),
            results2.iter().map(|r| (r.rule.as_str(), r.passed)).collect::<Vec<_>>(),
        );
    }
}
