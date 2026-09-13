use bozzard_scene::{
    GameFlowSettings, GamePhase, GameSession, ScreenText, TextAlignment, TextRendering,
};

/// Shared layout for native and embedded Play; hit areas come from GameSession::buttons.
pub fn game_menu(
    settings: &GameFlowSettings,
    session: &GameSession,
    size: [f32; 2],
) -> Vec<bozzard_render::DrawItem> {
    let mut items = Vec::new();
    let mut label = |text: String, anchor, offset, font_size, color| {
        let mut text = TextRendering {
            text,
            screen: Some(ScreenText { anchor, offset }),
            font_size,
            max_width: if font_size == 20. {
                None
            } else {
                Some((size[0] - 40.).clamp(40., 560.))
            },
            alignment: TextAlignment::Center,
            color,
            ..Default::default()
        };
        let mut shadow = text.clone();
        shadow.color = [0., 0., 0., 1.];
        for delta in [[-2., 0.], [2., 0.], [0., -2.], [0., 2.]] {
            shadow.screen.as_mut().unwrap().offset = [offset[0] + delta[0], offset[1] + delta[1]];
            items.push(crate::text_item(glam::Mat4::IDENTITY, &shadow));
        }
        // Button labels fit on one line so their visual bounds match the hit area.
        if font_size == 20. {
            text.max_width = None;
        }
        items.push(crate::text_item(glam::Mat4::IDENTITY, &text));
    };
    if !matches!(session.phase, GamePhase::Playing | GamePhase::Quit) {
        let title = match session.phase {
            GamePhase::Ready => settings.title.as_str(),
            GamePhase::Paused => "Paused",
            _ => "Game over",
        };
        label(title.into(), [0.5, 0.5], [0., -150.], 32., [1.; 4]);
        let message = if session.phase == GamePhase::Ready {
            &settings.instructions
        } else {
            &session.message
        };
        label(
            message.clone(),
            [0.5, 0.5],
            [0., -84.],
            18.,
            [0.9, 0.95, 1., 1.],
        );
    }
    for button in session.buttons() {
        label(
            button.label.into(),
            button.anchor,
            [button.offset[0], button.offset[1] - 12.],
            20.,
            [0.65, 1., 0.8, 1.],
        );
    }
    items
}
