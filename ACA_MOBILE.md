# Academy Audio Lab compact presentation

This fork adds a runtime phone projection; it never replaces the saved Edit/Mix dock trees.
Workspace Auto enters compact below 900 logical points wide or 600 high and returns only at
960 × 640. An active gesture defers switching. Workspace selection is stored independently in
the browser key `aca:audio:layout:v1`; drawer, touch and active-pane state are transient.

The compact session retains the existing waveform renderer and command dispatch. Its command
strip pins Open, Save, Undo/Redo, Play/Pause/Stop, position and Edit/Navigate. Tools, Clips,
Tracks, mixer and zoom remain available. Native track columns/rulers are projected more
compactly without editing the session's desktop view settings or track heights.

Touch starts in the editor body take a copy-on-write document/history checkpoint. A second
finger or cancellation restores that checkpoint before egui can finish the edit. Navigation
owns the contacts until all lift. Focus loss cancels; a replaced document invalidates the
old checkpoint. Audio buffers remain shared, and transport is not rolled back. Autosave
waits for the pending touch gesture to complete.

`academyLayoutState()` returns read-only applied presentation and essential command rectangles;
it exposes no project data or game commands. Existing theme/readiness interfaces remain.

Verification is recorded by the Academy integration owner. Source policy tests and engine
checkpoint regressions do not establish real iPhone/Android gesture, file-picker, memory or
listening acceptance. Those require the actual built browser app and physical devices.
