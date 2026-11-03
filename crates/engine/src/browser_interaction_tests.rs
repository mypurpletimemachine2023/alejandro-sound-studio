//! Touch cancellation must restore both artwork and history, rather than undo one arbitrary edit.
use crate::Engine;
use serde_json::json;

#[test]
fn browser_compact_checkpoint_restores_dirty_content_and_existing_redo() {
    let mut engine = Engine::default();
    engine.execute("track.new", &json!({"count": 1})).unwrap();
    engine.mark_clean();
    engine.execute("track.new", &json!({"count": 1})).unwrap();
    assert!(engine.undo());
    let before = serde_json::to_value(engine.session()).unwrap();
    let undo = engine.undo_label().map(str::to_string);
    let redo = engine.redo_label().map(str::to_string);
    let checkpoint = engine.interaction_checkpoint();
    engine.execute("track.new", &json!({"count": 3})).unwrap();
    assert!(!engine.can_redo());
    assert!(engine.is_dirty());
    engine.cancel_interaction(checkpoint);
    assert_eq!(serde_json::to_value(engine.session()).unwrap(), before);
    assert_eq!(engine.undo_label(), undo.as_deref());
    assert_eq!(engine.redo_label(), redo.as_deref());
    assert!(!engine.is_dirty());
    assert!(engine.redo());
    assert_eq!(engine.session().tracks.len(), 2);
}

#[test]
fn browser_compact_checkpoint_keeps_preexisting_unsaved_work_and_transport() {
    let mut engine = Engine::default();
    engine.execute("track.new", &json!({"count": 2})).unwrap();
    let before = serde_json::to_value(engine.session()).unwrap();
    let checkpoint = engine.interaction_checkpoint();
    engine.execute("track.new", &json!({"count": 1})).unwrap();
    engine.transport.position = 1000;
    engine.cancel_interaction(checkpoint);
    assert_eq!(serde_json::to_value(engine.session()).unwrap(), before);
    assert!(engine.is_dirty());
    assert_eq!(engine.transport.position, 1000);
    assert!(engine.undo());
    assert!(engine.session().tracks.is_empty());
}
