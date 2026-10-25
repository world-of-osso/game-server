//! Process-isolated private-file handshake through actual Trading Post FIFO work.
use super::*;
use std::{
    fs,
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

const CHILD_ROOT: &str = "TP_NATIVE_HOLD_FIXTURE_CHILD";
const HOLD_ROOT: &str = "NATIVE_PROOF_HOLD_PERSISTENCE_DIR";

struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        match self.0.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) => {}
            Err(error) => eprintln!("native hold fixture status: {error}"),
        }
        if let Err(error) = self.0.kill() {
            eprintln!("native hold fixture cleanup kill: {error}");
        }
        if let Err(error) = self.0.wait() {
            eprintln!("native hold fixture cleanup wait: {error}");
        }
    }
}

fn wait_for_file(root: &Path, name: &str, seconds: u64) -> bool {
    let deadline = Instant::now() + Duration::from_secs(seconds);
    while Instant::now() < deadline {
        if root.join(name).exists() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    false
}

fn wait_for_hold_or_completion(root: &Path) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if root.join("held").exists() {
            return true;
        }
        if root.join("completed").exists() {
            return false;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    false
}

fn prepare_hold_buyer(root: &Path) -> Harness {
    let mut h =
        crate::bank_test_harness::harness_at(&root.join("players.redb"), "Holdbuyer").unwrap();
    prepare(&mut h, 1790812900);
    open_tawny(&mut h);
    store::grant_tender(h.db(), ACCOUNT, 500).unwrap();
    assert_eq!(request(&mut h, TradingPostRequest::Query).tender, 500);
    fs::write(root.join("ready"), b"ready").unwrap();
    assert!(
        wait_for_file(root, "go", 60),
        "parent starts the private proof request"
    );
    assert_eq!(request(&mut h, TradingPostRequest::Balance).tender, 500);
    assert!(
        root.join("arm").exists(),
        "Balance cannot consume the purchase hold"
    );
    assert!(!root.join("held").exists());
    h
}

fn observe_held_purchase(h: &Harness, root: &Path) {
    let observed = serde_json::json!({
        "tender": store::load(h.db(), ACCOUNT).tender,
        "appearance": crate::persistence::appearance_store::load_account_appearances(h.db(), ACCOUNT).has(37866),
    });
    fs::write(
        root.join("observed"),
        serde_json::to_vec(&observed).unwrap(),
    )
    .unwrap();
}

fn receive_hold_purchase(
    h: &mut Harness,
    root: &Path,
    command: TradingPostRequest,
) -> TradingPostSnapshot {
    h.app
        .world_mut()
        .get_mut::<MessageSender<TradingPostRequest>>(h.connection)
        .unwrap()
        .send::<InstanceChannel>(command);
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut held_frames = 0;
    loop {
        h.app.update();
        let mut replies: Vec<_> = h
            .app
            .world_mut()
            .get_mut::<MessageReceiver<TradingPostSnapshot>>(h.connection)
            .unwrap()
            .receive()
            .collect();
        if let Some(reply) = replies.pop() {
            return reply;
        }
        if root.join("held").exists() {
            held_frames += 1;
            if held_frames == 2 {
                observe_held_purchase(h, root);
            }
        }
        assert!(
            Instant::now() < deadline,
            "actual Trading Post reply completes within the fixture bound"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn child_hold_case(root: &Path, checkout: bool, timeout: bool) {
    let mut h = prepare_hold_buyer(root);
    let command = if checkout {
        TradingPostRequest::Checkout {
            vendor_item_ids: vec![4],
        }
    } else {
        TradingPostRequest::Purchase { vendor_item_id: 4 }
    };
    let kind = command.reply_kind();
    let reply = receive_hold_purchase(&mut h, root, command);
    assert_eq!(reply.reply_kind, kind);
    let expected_tender = if timeout { 500 } else { 400 };
    if timeout {
        assert!(reply.error.as_ref().unwrap().contains("deadline expired"));
        assert_eq!(store::load(h.db(), ACCOUNT).tender, 500);
    } else {
        assert!(reply.error.is_none(), "{:?}", reply.error);
        assert!(
            reply
                .items
                .iter()
                .find(|row| row.id == 4)
                .unwrap()
                .purchased
        );
    }
    assert_eq!(reply.tender, expected_tender);
    assert_eq!(
        crate::persistence::appearance_store::load_account_appearances(h.db(), ACCOUNT).has(37866),
        !timeout
    );
    assert_eq!(
        request(&mut h, TradingPostRequest::Balance).tender,
        expected_tender,
        "later FIFO work drains after release or watchdog refusal"
    );
    fs::write(root.join("completed"), b"completed").unwrap();
}

fn wait_for_child(child: &mut Child) -> std::process::ExitStatus {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        if Instant::now() >= deadline {
            child
                .kill()
                .expect("kill only the owned hung fixture child");
            return child.wait().expect("reap owned fixture child");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn run_hold_case(test: &str, checkout: bool, timeout: bool) {
    let expect_hold = cfg!(feature = "native-proof-hold-persistence");
    if let Some(root) = std::env::var_os(CHILD_ROOT) {
        child_hold_case(Path::new(&root), checkout, timeout);
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let log = fs::File::create(root.join("child.log")).unwrap();
    let mut child = OwnedChild(
        Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test, "--nocapture"])
            .env(CHILD_ROOT, root)
            .env(HOLD_ROOT, root)
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap(),
    );
    let ready = wait_for_file(root, "ready", 60);
    fs::write(root.join("arm"), b"one purchase").unwrap();
    fs::write(root.join("go"), b"go").unwrap();
    let held = ready && wait_for_hold_or_completion(root);
    let observed = held && wait_for_file(root, "observed", 5);
    // Watchdog proof deliberately sends no release; its request must fail without charge.
    if !timeout {
        fs::write(root.join("release"), b"release").unwrap();
    }
    let status = wait_for_child(&mut child.0);
    let log = fs::read_to_string(root.join("child.log")).unwrap();
    assert!(
        ready && status.success(),
        "child must execute the real transaction successfully: {log}"
    );
    assert_eq!(
        held, expect_hold,
        "only the explicit private feature may claim a hold: {log}"
    );
    if expect_hold {
        assert!(
            observed,
            "the app must keep ticking while its durable purchase is held"
        );
        let observation: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join("observed")).unwrap()).unwrap();
        assert_eq!(observation["tender"], 500);
        assert_eq!(observation["appearance"], false);
        for marker in ["arm", "held", "release"] {
            assert!(!root.join(marker).exists());
        }
    } else {
        assert!(
            root.join("arm").exists(),
            "default binary ignores the private hook entirely"
        );
    }
    assert!(root.join("completed").exists());
}

#[cfg(feature = "native-proof-hold-persistence")]
#[test]
fn native_proof_hold_persistence_purchase_waits_for_release() {
    run_hold_case(
        "trading_post::tests::native_proof_hold::native_proof_hold_persistence_purchase_waits_for_release",
        false,
        false,
    );
}

#[cfg(feature = "native-proof-hold-persistence")]
#[test]
fn native_proof_hold_persistence_checkout_waits_for_release() {
    run_hold_case(
        "trading_post::tests::native_proof_hold::native_proof_hold_persistence_checkout_waits_for_release",
        true,
        false,
    );
}

#[cfg(not(feature = "native-proof-hold-persistence"))]
#[test]
fn native_proof_hold_persistence_purchase_default_ignores_private_arm() {
    run_hold_case(
        "trading_post::tests::native_proof_hold::native_proof_hold_persistence_purchase_default_ignores_private_arm",
        false,
        false,
    );
}

#[cfg(not(feature = "native-proof-hold-persistence"))]
#[test]
fn native_proof_hold_persistence_checkout_default_ignores_private_arm() {
    run_hold_case(
        "trading_post::tests::native_proof_hold::native_proof_hold_persistence_checkout_default_ignores_private_arm",
        true,
        false,
    );
}

#[cfg(feature = "native-proof-hold-persistence")]
#[test]
fn native_proof_hold_persistence_timeout_refuses_purchase_without_charge() {
    run_hold_case(
        "trading_post::tests::native_proof_hold::native_proof_hold_persistence_timeout_refuses_purchase_without_charge",
        false,
        true,
    );
}
