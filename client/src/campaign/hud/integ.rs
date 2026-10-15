//! Glue between the camera hotkeys and the panels (integration lot).
//!
//! `FocusCamera` is consumed here to open what the camera lands on, as `camera_controller.gd` +
//! `campaign_ui_controller.gd` do: `Home` / `Ctrl+Tab` focus a province (capital, a controlled
//! province) and open its panel; `Tab` focuses an idle army and selects it (the army panel opens
//! from the selection).

use bevy::prelude::*;
use sim_campaign::ArmyId;

use crate::campaign::armies::SelectedArmy;
use crate::campaign::camera::FocusCamera;
use crate::campaign::hud::HudSet;
use crate::campaign::panels::OpenPanel;
use crate::campaign::provinces::picker::SelectProvince;

/// Province or army focus requests → selection + panel.
pub fn focus_opens_panels(
    mut focus: MessageReader<FocusCamera>,
    mut panels: MessageWriter<OpenPanel>,
    mut select_province: MessageWriter<SelectProvince>,
    mut selected_army: ResMut<SelectedArmy>,
) {
    for request in focus.read() {
        match request {
            FocusCamera::Province(id) => {
                select_province.write(SelectProvince(Some(id.clone())));
                panels.write(OpenPanel::Province(id.clone()));
            }
            FocusCamera::Army(id) => {
                if let Some(id) = ArmyId::parse(id)
                    && selected_army.0.as_ref() != Some(&id)
                {
                    selected_army.0 = Some(id);
                }
            }
            FocusCamera::Point(_) | FocusCamera::Follow(_) => {}
        }
    }
}

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        focus_opens_panels
            .in_set(HudSet::Ui)
            .run_if(super::flow::orders_open),
    );
}
