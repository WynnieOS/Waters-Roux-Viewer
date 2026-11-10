# Waters Roux — In-world editing release gate

**Priority: P0.** Owner's Beta 27 report: the viewer logs in but selected/edited objects are not visually clear enough to use like Firestorm. This blocks any stable release, even if toolbar buttons are enabled.

## Source investigation
- `crates/aurora-viewer/src/build/mod.rs`: `BuildTool::draw_overlay`, `outlined`, `wire_selection`, `silhouette_selection`, `face_selection`, `manipulator_at`, and `set_transform`.
- The 3D selection highlight and gizmo are drawn with `draw::Painter3d` on an egui Background layer; verify the viewer's actual scene composition and floaters' ordering before switching layers.
- Check whether pick-ray generation, pointer pixel/point conversion and gizmo render projection use the same camera and viewport. Test HiDPI at 100%, 125%, and 150%.
- Check linked objects, mesh/sculpt/flexible objects, alpha surfaces, HUDs/attachments, object rights, selection-distance preferences, and simulator position acknowledgements.
- Keep legitimate permission/restriction-based disabled states. Do **not** make every button globally active regardless of state.

## Acceptance criteria (real Windows Second Life session)
- [ ] Object click selects the correct prim/linkset and renders an unmistakable highlight; clearing the selection clears it.
- [ ] Edit Linked and Select Face display the correct child/face; mesh, sculpts, flexis and alpha objects remain selectable.
- [ ] Move/Rotate/Stretch manipulators are visible and axis hitboxes match the rendered handles at ordinary zoom and different display scaling.
- [ ] Drag gestures and modifier keys perform the matching operation without camera/pose conflicts, and the simulator agrees with the final edit.
- [ ] Position/rotation/size fields preview and commit independently, and each permission gate checks the matching operation.
- [ ] Selection/gizmo appearance settings can be adjusted and persisted without hiding edit controls by accident.
- [ ] No edit bypasses RLVa, simulator object permissions, parcel rights or ownership checks.
- [ ] Tests cover pick-ray/overlay camera coordinate mapping, transform-operation selection, and editing failure paths.
- [ ] Windows CI passes and the owner verifies in-world editing before this gate is checked complete.

## Related work
PR #5 connects 16 toolbar commands but **does not fix picking/rendered editing visibility**. PR #4 is separate login appearance work. Both remain draft until their own CI and in-world checks. Native POV recording, nametag visual customization and enhanced consensual multi-avatar posing are separate implementation work, not solved by this editing gate.

## Notes on the rotation-mode patch
A typed Object-editor rotation-only update was passed through `can_transform(..., EditMode::Move)`. The source now selects `EditMode::Rotate` when rotation is the operation, without changing the existing stretch-first precedence for combined edits. This is a code correction, **not** proof that overlays are fixed or that a Windows build has been tested.
