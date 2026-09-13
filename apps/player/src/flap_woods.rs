//! Export acceptance route through the player's real physical-key dispatch.
//! This is CPU verification; native presentation is checked separately.
use super::*;
use bozzard_scene::{GamePhase as P, TextRendering};

fn press(player: &mut Player, code: KeyCode) -> Result<()> {
    for state in [ElementState::Pressed, ElementState::Released] {
        player.dispatch_keyboard(
            PhysicalKey::Code(code),
            &Key::Unidentified(winit::keyboard::NativeKey::Unidentified),
            state,
            false,
            false,
        )?;
    }
    Ok(())
}
fn tick(player: &mut Player) -> Result<()> {
    player.demo.app.step();
    player.demo.check_simulation()
}
fn position(player: &Player, id: &str) -> Result<[f32; 3]> {
    let entity = player
        .demo
        .instance()
        .entity(id)
        .context("missing Flap Woods object")?;
    Ok(player
        .demo
        .app
        .world
        .get::<Transform>(entity)
        .context("missing transform")?
        .translation)
}
fn score(player: &Player) -> Result<&str> {
    let entity = player
        .demo
        .instance()
        .entity("score")
        .context("missing score HUD")?;
    Ok(&player
        .demo
        .app
        .world
        .get::<TextRendering>(entity)
        .context("missing score text")?
        .text)
}
fn phase(player: &Player) -> Result<P> {
    Ok(player
        .demo
        .game_session()
        .context("Flap Woods verification requires Game Flow")?
        .phase)
}

pub fn verify(player: &mut Player) -> Result<()> {
    ensure!(
        phase(player)? == P::Ready && score(player)? == "Score: 0",
        "game must open ready with zero score"
    );
    let initial = player.demo.instance().capture(&player.demo.app.world)?;
    player.gameplay_controls.event(&WindowEvent::Focused(true));
    press(player, KeyCode::Space)?;
    for _ in 0..30 {
        tick(player)?;
    }
    ensure!(
        player.demo.instance().capture(&player.demo.app.world)? == initial,
        "ready game advanced"
    );
    press(player, KeyCode::Enter)?;
    ensure!(phase(player)? == P::Playing, "Enter did not start");
    // Follow the first gap using only physical flap input. No world mutations.
    for _ in 0..250 {
        if position(player, "bird")?[1] < -0.4 {
            press(player, KeyCode::Space)?;
        }
        tick(player)?;
        ensure!(
            phase(player)? == P::Playing,
            "flap route died before clearing the first pipe"
        );
    }
    ensure!(
        score(player)? == "Score: 1",
        "first cleared pipe did not score exactly once: {}",
        score(player)?
    );
    press(player, KeyCode::Escape)?;
    ensure!(phase(player)? == P::Paused, "Escape did not pause");
    let paused = player.demo.instance().capture(&player.demo.app.world)?;
    press(player, KeyCode::Space)?;
    for _ in 0..60 {
        tick(player)?;
    }
    ensure!(
        player.demo.instance().capture(&player.demo.app.world)? == paused,
        "paused world changed"
    );
    press(player, KeyCode::Escape)?;
    ensure!(phase(player)? == P::Playing, "Escape did not resume");
    for _ in 0..180 {
        tick(player)?;
    }
    ensure!(
        phase(player)? == P::GameOver && score(player)? == "Score: 1",
        "death must retain final score"
    );
    let ended = player.demo.instance().capture(&player.demo.app.world)?;
    for _ in 0..60 {
        tick(player)?;
    }
    ensure!(
        player.demo.instance().capture(&player.demo.app.world)? == ended,
        "game-over world changed"
    );
    press(player, KeyCode::Enter)?;
    ensure!(
        phase(player)? == P::Playing
            && player.demo.instance().capture(&player.demo.app.world)? == initial,
        "retry did not reset the complete game"
    );
    press(player, KeyCode::Space)?;
    for _ in 0..20 {
        tick(player)?;
    }
    ensure!(
        position(player, "bird")?[1] > 1.,
        "retry did not restore flap input"
    );
    press(player, KeyCode::KeyR)?;
    ensure!(
        player.demo.instance().capture(&player.demo.app.world)? == initial,
        "R restart did not reset"
    );
    press(player, KeyCode::Escape)?;
    press(player, KeyCode::KeyQ)?;
    ensure!(phase(player)? == P::Quit, "Quit did not end the session");
    println!(
        "flap_woods_ok ready=true score=1 paused=true game_over=true retry=true restart=true quit=true"
    );
    Ok(())
}
