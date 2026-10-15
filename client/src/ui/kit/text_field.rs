//! Single-line text field: click to focus, UTF-8 typing (accents come through
//! `KeyboardInput::text`), cursor, Backspace/Delete/arrows/Home/End, Enter submits, Escape drops
//! the focus.
//!
//! - [`text_field`] builds the widget; read [`TextFieldChanged`] / [`TextFieldSubmitted`]
//!   (or the [`TextField`] component) in the owning screen.
//! - [`TextInputFocus`]: public resource. Other screens' keyboard shortcuts must stay quiet while
//!   `focus.is_active()` is true (`.run_if(not(text_input_active))` or an explicit test).
//! - Escape while a field has the focus only drops the focus: `KeyEdges::escape` is cleared (the
//!   system runs before `pause_escape`) and the panel stack ignores that Escape.
//! - Script hook: `CENT_ANS_TYPE=<text>` types the text, one character per frame, once a field
//!   has the focus (`click <FieldName>` focuses a field). A `type <text>` agent command would
//!   only need to write `KeyboardInput { text: Some(..) }` messages, like [`env_typing`].

use bevy::ecs::system::SystemParam;
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;

use super::style::{self, UiType};
use super::widgets::KitWidget;
use super::{ButtonPressed, UiTheme};

/// State of a text field. `cursor` counts characters, not bytes.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct TextField {
    pub value: String,
    pub cursor: usize,
    pub placeholder: String,
    /// Maximum number of characters.
    pub max_chars: usize,
}

impl TextField {
    pub fn new(value: &str, placeholder: &str, max_chars: usize) -> Self {
        let value: String = value.chars().take(max_chars).collect();
        Self {
            cursor: value.chars().count(),
            value,
            placeholder: placeholder.to_owned(),
            max_chars,
        }
    }

    fn byte_at(&self, chars: usize) -> usize {
        self.value
            .char_indices()
            .nth(chars)
            .map_or(self.value.len(), |(at, _)| at)
    }

    /// Inserts printable characters at the cursor; true when the value changed.
    pub fn insert(&mut self, text: &str) -> bool {
        let mut changed = false;
        for c in text.chars().filter(|c| !c.is_control()) {
            if self.value.chars().count() >= self.max_chars {
                break;
            }
            let at = self.byte_at(self.cursor);
            self.value.insert(at, c);
            self.cursor += 1;
            changed = true;
        }
        changed
    }

    pub fn backspace(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        let at = self.byte_at(self.cursor);
        self.value.remove(at);
        true
    }

    pub fn delete(&mut self) -> bool {
        if self.cursor >= self.value.chars().count() {
            return false;
        }
        let at = self.byte_at(self.cursor);
        self.value.remove(at);
        true
    }

    pub fn move_by(&mut self, delta: i32) {
        let len = self.value.chars().count() as i32;
        self.cursor = (self.cursor as i32 + delta).clamp(0, len) as usize;
    }

    pub fn set(&mut self, value: &str) {
        self.value = value.chars().take(self.max_chars).collect();
        self.cursor = self.value.chars().count();
    }

    /// Text shown in the field: the value with a `|` at the cursor while focused.
    pub fn display(&self, focused: bool) -> String {
        if !focused {
            return self.value.clone();
        }
        let at = self.byte_at(self.cursor);
        format!("{}|{}", &self.value[..at], &self.value[at..])
    }
}

/// Which field has the keyboard (public: other lots test it to ignore their shortcuts).
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TextInputFocus(pub Option<Entity>);

impl TextInputFocus {
    pub fn is_active(&self) -> bool {
        self.0.is_some()
    }
}

/// Run condition: no text field has the focus (`.run_if(not(text_input_active))` is the inverse).
pub fn text_input_active(focus: Option<Res<TextInputFocus>>) -> bool {
    focus.is_some_and(|f| f.is_active())
}

/// Run condition: keyboard shortcuts may run.
pub fn keyboard_free(focus: Option<Res<TextInputFocus>>) -> bool {
    !focus.is_some_and(|f| f.is_active())
}

/// The value changed (typing, Backspace, Delete).
#[derive(Message, Debug, Clone)]
pub struct TextFieldChanged {
    pub entity: Entity,
    pub name: String,
    pub text: String,
}

/// Enter was pressed in the field.
#[derive(Message, Debug, Clone)]
pub struct TextFieldSubmitted {
    pub entity: Entity,
    pub name: String,
    pub text: String,
}

#[derive(Component)]
struct FieldText;

/// A field named `name` (script: `click <name>` focuses it), 64 characters at most.
pub fn text_field(
    theme: &UiTheme,
    name: &str,
    value: &str,
    placeholder: &str,
    width: f32,
) -> impl Bundle {
    text_field_limited(theme, name, value, placeholder, width, 64)
}

/// Same, with a custom character limit.
pub fn text_field_limited(
    theme: &UiTheme,
    name: &str,
    value: &str,
    placeholder: &str,
    width: f32,
    max_chars: usize,
) -> impl Bundle {
    (
        Name::new(name.to_owned()),
        KitWidget("text_field"),
        TextField::new(value, placeholder, max_chars),
        Hovered::default(),
        Node {
            width: Val::Px(width),
            min_height: Val::Px(30.0),
            padding: theme.inner_padding("inset", 4.0),
            align_items: AlignItems::Center,
            overflow: Overflow::clip(),
            ..default()
        },
        theme.sliced("inset"),
        children![(
            FieldText,
            Text::new(value),
            theme.text_font(false, false, UiType::Body.size()),
            TextColor(style::INK),
        )],
    )
}

pub struct TextFieldPlugin;

impl Plugin for TextFieldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TextInputFocus>()
            .add_message::<TextFieldChanged>()
            .add_message::<TextFieldSubmitted>()
            .add_systems(
                Update,
                (focus_fields, env_typing, type_into_field, refresh_fields)
                    .chain()
                    .before(crate::menu::pause::pause_escape),
            );
    }
}

#[derive(SystemParam)]
struct FieldFocus<'w> {
    focus: ResMut<'w, TextInputFocus>,
}

/// Focus by mouse click (or `click <Name>`), blur by clicking elsewhere; a focus whose field was
/// despawned is dropped.
fn focus_fields(
    mouse: Res<ButtonInput<MouseButton>>,
    fields: Query<(Entity, &Hovered), With<TextField>>,
    mut presses: MessageReader<ButtonPressed>,
    mut state: FieldFocus,
) {
    if let Some(entity) = state.focus.0
        && fields.get(entity).is_err()
    {
        state.focus.0 = None;
    }
    for press in presses.read() {
        if fields.get(press.entity).is_ok() {
            state.focus.0 = Some(press.entity);
        }
    }
    if mouse.just_pressed(MouseButton::Left) {
        let hit = fields
            .iter()
            .find(|(_, hovered)| hovered.get())
            .map(|(entity, _)| entity);
        state.focus.0 = hit;
    }
}

/// Dev hook: `CENT_ANS_TYPE=<text>` types one character per frame into the focused field.
fn env_typing(
    mut pending: Local<Option<Vec<char>>>,
    mut started: Local<bool>,
    focus: Res<TextInputFocus>,
    mut events: MessageWriter<KeyboardInput>,
    windows: Query<Entity, With<bevy::window::PrimaryWindow>>,
) {
    if !*started {
        *started = true;
        *pending = std::env::var("CENT_ANS_TYPE")
            .ok()
            .map(|text| text.chars().collect());
    }
    let Some(queue) = pending.as_mut() else {
        return;
    };
    if !focus.is_active() || queue.is_empty() {
        return;
    }
    let c = queue.remove(0);
    events.write(KeyboardInput {
        key_code: KeyCode::Unidentified(bevy::input::keyboard::NativeKeyCode::Unidentified),
        logical_key: Key::Unidentified(NativeKey::Unidentified),
        state: ButtonState::Pressed,
        text: Some(c.to_string().into()),
        repeat: false,
        window: windows.iter().next().unwrap_or(Entity::PLACEHOLDER),
    });
}

#[allow(clippy::too_many_arguments)]
fn type_into_field(
    mut keys: MessageReader<KeyboardInput>,
    mut state: FieldFocus,
    mut fields: Query<(&mut TextField, &Name)>,
    mut edges: ResMut<crate::menu::KeyEdges>,
    mut changed: MessageWriter<TextFieldChanged>,
    mut submitted: MessageWriter<TextFieldSubmitted>,
) {
    let Some(entity) = state.focus.0 else {
        keys.clear();
        return;
    };
    let Ok((mut field, name)) = fields.get_mut(entity) else {
        return;
    };
    let mut edited = false;
    for key in keys.read() {
        if key.state != ButtonState::Pressed {
            continue;
        }
        // Script keys carry only a key code; real keys carry both.
        let named = match (&key.logical_key, key.key_code) {
            (Key::Unidentified(_), KeyCode::Backspace) => Key::Backspace,
            (Key::Unidentified(_), KeyCode::Delete) => Key::Delete,
            (Key::Unidentified(_), KeyCode::ArrowLeft) => Key::ArrowLeft,
            (Key::Unidentified(_), KeyCode::ArrowRight) => Key::ArrowRight,
            (Key::Unidentified(_), KeyCode::Home) => Key::Home,
            (Key::Unidentified(_), KeyCode::End) => Key::End,
            (Key::Unidentified(_), KeyCode::Enter) => Key::Enter,
            (Key::Unidentified(_), KeyCode::Escape) => Key::Escape,
            (other, _) => other.clone(),
        };
        match &named {
            Key::Backspace => edited |= field.backspace(),
            Key::Delete => edited |= field.delete(),
            Key::ArrowLeft => field.move_by(-1),
            Key::ArrowRight => field.move_by(1),
            Key::Home => field.move_by(i32::MIN / 2),
            Key::End => field.move_by(i32::MAX / 2),
            Key::Enter => {
                submitted.write(TextFieldSubmitted {
                    entity,
                    name: name.to_string(),
                    text: field.value.clone(),
                });
            }
            Key::Escape => {
                state.focus.0 = None;
                edges.escape = false;
            }
            _ => {
                if let Some(text) = &key.text {
                    edited |= field.insert(text);
                }
            }
        }
    }
    if edited {
        changed.write(TextFieldChanged {
            entity,
            name: name.to_string(),
            text: field.value.clone(),
        });
    }
    // While a field has the focus Escape belongs to it, also on the frame it was dropped.
    if state.focus.0.is_some() {
        edges.escape = false;
    }
}

fn refresh_fields(
    focus: Res<TextInputFocus>,
    theme: Res<UiTheme>,
    fields: Query<(Entity, &TextField, &Children)>,
    mut texts: Query<(&mut Text, &mut TextColor), With<FieldText>>,
) {
    for (entity, field, children) in &fields {
        let focused = focus.0 == Some(entity);
        let empty = field.value.is_empty() && !focused;
        let (shown, ink) = if empty {
            (field.placeholder.clone(), style::TAB_UNSELECTED_INK)
        } else {
            (field.display(focused), style::INK)
        };
        for child in children.iter() {
            if let Ok((mut text, mut color)) = texts.get_mut(child) {
                if text.0 != shown {
                    text.0 = shown.clone();
                }
                if color.0 != ink {
                    color.0 = ink;
                }
            }
        }
    }
    let _ = &theme;
}

pub fn dump(world: &World) -> serde_json::Value {
    let focus = world.get_resource::<TextInputFocus>().and_then(|f| f.0);
    let mut query = world.try_query::<(Entity, &Name, &TextField)>().unwrap();
    let fields: Vec<_> = query
        .iter(world)
        .map(|(entity, name, field)| {
            serde_json::json!({
                "name": name.as_str(), "value": field.value, "cursor": field.cursor,
                "focused": focus == Some(entity),
            })
        })
        .collect();
    serde_json::json!({"focus_active": focus.is_some(), "fields": fields})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typing_accents_and_cursor() {
        let mut field = TextField::new("", "", 10);
        assert!(field.insert("é"));
        assert!(field.insert("té"));
        assert_eq!(field.value, "été");
        field.move_by(-1);
        assert!(field.insert("x"));
        assert_eq!(field.value, "étxé");
        assert_eq!(field.display(true), "étx|é");
        assert!(field.backspace());
        assert_eq!(field.value, "été");
        field.move_by(i32::MIN / 2);
        assert!(!field.backspace());
        assert!(field.delete());
        assert_eq!(field.value, "té");
    }

    #[test]
    fn limit_and_control_chars() {
        let mut field = TextField::new("ab", "", 3);
        assert!(field.insert("c\nd"));
        assert_eq!(field.value, "abc");
        assert!(!field.insert("\t"));
        field.set("long text");
        assert_eq!(field.value, "lon");
        assert_eq!(field.cursor, 3);
    }

    #[test]
    fn display_unfocused_has_no_cursor() {
        assert_eq!(TextField::new("Jeanne", "", 20).display(false), "Jeanne");
    }
}
