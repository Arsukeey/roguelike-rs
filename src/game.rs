use crate::ai;
use crate::curses::{Graphics, INV_X, PLAYER, WINDOW_HEIGHT, WINDOW_WIDTH};
use crate::object::{move_by, Fighter, Object};
use crate::tile;
use crate::tile::{Map, Tile, MAP_HEIGHT, MAP_WIDTH};
use pancurses::{Input, Window};

const PLAYER_DEF_HP: i32 = 30;

pub struct Game {
    pub map: Map,
    pub graphics: Graphics,
    pub inventory: Vec<Object>,
}

impl Game {
    pub fn start(&mut self) {
        let mut player = Object::new(
            WINDOW_WIDTH / 2,
            WINDOW_HEIGHT / 2,
            '@',
            pancurses::COLOR_WHITE,
            true,
            "player",
            true,
        );

        player.alive = true;

        player.fighter = Some(Fighter {
            max_hp: PLAYER_DEF_HP,
            hp: PLAYER_DEF_HP,
            defence: 2,
            power: 5,
        });

        self.graphics.push_obj(player);

        // procedurally generate the map
        self.map = tile::make_map(&mut self.graphics.objects.borrow_mut());

        let mut frames = 1;
        loop {
            frames += 1;

            self.graphics.add_status(self.get_names_under_player(), 1);

            self.graphics.draw(&self.map);

            self.graphics
                .draw_player_stats(&mut self.graphics.objects.borrow_mut()[PLAYER]);

            self.show_inventory();

            // regen every n moves
            if frames % 4 == 0 {
                frames = 0;
                let player_hp = self.graphics.objects.borrow()[PLAYER]
                    .clone()
                    .fighter
                    .unwrap()
                    .hp;
                if player_hp < PLAYER_DEF_HP {
                    self.graphics.objects.borrow_mut()[PLAYER]
                        .fighter
                        .as_mut()
                        .unwrap()
                        .hp += 1;
                }
            }

            let player_action = self.handle_keys();

            if let PlayerAction::Exit = player_action {
                break;
            }

            if self.graphics.objects.borrow()[PLAYER].alive
                && player_action != PlayerAction::DidntTakeTurn
            {
                for object in &*self.graphics.objects.borrow() {
                    // only if object is not player
                    if (object as *const _) != (&self.graphics.objects.borrow()[PLAYER] as *const _)
                    {
                        /*
                        self.graphics
                            .statuses
                            .push(Status::new(format!("The {} growls!", object.name), 1));
                        */
                    }
                }

                let m = self.graphics.objects.borrow().len();
                for id in 0..m {
                    if self.graphics.objects.borrow()[id].ai.is_some() {
                        ai::take_turn(id, self);
                    }
                }
            }
        }
    }

    fn get_names_under_player(&self) -> String {
        let objs = self.graphics.objects.borrow();
        let (px, py) = objs[PLAYER].pos();

        let names = objs
            .iter()
            .filter(|obj| obj.pos() == (px, py) && obj.name != "player")
            .map(|obj| obj.name.clone())
            .collect::<Vec<_>>();

        names.join(", ")
    }

    pub fn handle_keys(&mut self) -> PlayerAction {
        let is_alive = self.graphics.objects.borrow()[PLAYER].alive;
        match (self.graphics.window.getch(), is_alive) {
            (Some(Input::KeyDC), _) | (Some(Input::Character('q')), _) => PlayerAction::Exit, // exit game

            (Some(Input::Character(',')), true) => {
                // let objs = self.graphics.objects.borrow();
                let item_id = self.graphics.objects.borrow().iter().position(|object| {
                    object.pos() == self.graphics.objects.borrow()[PLAYER].pos()
                        && object.item.is_some()
                });

                if let Some(item_id) = item_id {
                    self.pick_item_up(item_id);
                }
                PlayerAction::TookTurn
            }

            // rest, do nothing for a turn
            (Some(Input::Character('.')), true) => PlayerAction::TookTurn,

            // movement keys
            (Some(Input::KeyUp), true) | (Some(Input::Character('k')), true) => {
                self.player_move_or_attack(0, -1);
                PlayerAction::TookTurn
            }
            (Some(Input::KeyDown), true) | (Some(Input::Character('j')), true) => {
                self.player_move_or_attack(0, 1);
                PlayerAction::TookTurn
            }
            (Some(Input::KeyLeft), true) | (Some(Input::Character('h')), true) => {
                self.player_move_or_attack(-1, 0);
                PlayerAction::TookTurn
            }
            (Some(Input::KeyRight), true) | (Some(Input::Character('l')), true) => {
                self.player_move_or_attack(1, 0);
                PlayerAction::TookTurn
            }

            (Some(_), _) | (None, _) => PlayerAction::DidntTakeTurn,
        }
    }

    /// add to the player's inventory and remove from the map
    pub fn pick_item_up(&mut self, object_id: usize) {
        if self.inventory.len() >= 26 {
            self.graphics.add_status(
                format!(
                    "Your inventory is full, cannot pick up {}.",
                    self.graphics.objects.borrow()[object_id].name
                ),
                1,
            );
        } else {
            let item = self.graphics.objects.borrow_mut().swap_remove(object_id);
            self.graphics
                .add_status(format!("You picked up a {}!", item.name), 1);
            self.inventory.push(item);
        }
    }

    pub fn player_move_or_attack(&mut self, dx: i32, dy: i32) {
        // the coordinates the player is moving to/attacking
        let x = self.graphics.objects.borrow()[PLAYER].x + dx;
        let y = self.graphics.objects.borrow()[PLAYER].y + dy;

        // try to find an attackable object there
        let target_id = self
            .graphics
            .objects
            .borrow_mut()
            .iter()
            .position(|object| object.pos() == (x, y) && object.alive);

        // attack if target found, move otherwise
        match target_id {
            Some(target_id) => {
                let mut objs = self.graphics.objects.borrow_mut();
                let (player, mut target) = ai::mut_two(PLAYER, target_id, &mut objs);

                player.attack(&mut target, &mut self.graphics.statuses);
            }
            None => {
                move_by(
                    PLAYER,
                    dx,
                    dy,
                    &self.map,
                    &mut self.graphics.objects.borrow_mut(),
                );
            }
        }
    }

    // ------------------------------------
    // inventory-related methods
    fn show_inventory(&self) {
        self.graphics.window.color_set(pancurses::COLOR_WHITE);
        self.graphics.window.mvaddstr(1, INV_X, "Inventory:");
        for (i, item) in self.inventory.iter().enumerate() {
            self.graphics.window.mvaddstr(
                (i + 3) as i32,
                INV_X,
                format!("{} - {}", (i + 97) as u8 as char, item.name.clone()),
            );
        }
    }

    fn apply_item(&self) {}

    fn discard_item(&self) {}
}

impl Default for Game {
    fn default() -> Self {
        Self {
            map: vec![vec![Tile::empty(); MAP_HEIGHT as usize]; MAP_WIDTH as usize],
            graphics: Graphics::default(),
            inventory: vec![],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}
