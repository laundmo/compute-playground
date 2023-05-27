// fn lerp(a: f32, b: f32, t: f32) -> f32 {
//     (1.0 - t) * a + t * b
// }

// fn inv_lerp(a: f32, b: f32, v: f32) -> f32 {
//     (v - a) / (b - a)
// }

// fn remap(i: Vec2, o: Vec2, v: f32) -> f32 {
//     lerp(o.x, o.y, inv_lerp(i.x, i.y, v))
// }

// fn zoom_smooth(
//     mut target: Local<Vec4>,
//     mut zoom_target: EventReader<ZoomEvent>,
//     mut shader_params: ResMut<ShaderParams>,
//     mut timer: Local<Timer>,
//     time: Res<Time>,
// ) {
//     if !zoom_target.is_empty() {
//         *timer = Timer::from_seconds(1.0, TimerMode::Once);
//         *target = zoom_target.iter().last().unwrap().0;
//     }
//     if timer.duration().as_secs_f32() != 0.0 && !timer.finished() {
//         // fix: only run when timer runs
//         timer.tick(time.delta());
//         shader_params.extents = shader_params.extents.lerp(*target, timer.percent());
//     }
// }
