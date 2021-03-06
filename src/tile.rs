use crate::map_gen;
use crate::map_gen::{Rect, MAX_ROOMS, ROOM_MAX_SIZE, ROOM_MIN_SIZE};
use crate::object::Object;
use crate::object_gen;
use rand::Rng;

pub const MAP_WIDTH: i32 = 100;
pub const MAP_HEIGHT: i32 = 30;

#[derive(Clone, Copy, Debug)]
pub struct Tile {
    pub blocked: bool,
    pub block_sight: bool,
    pub visible: bool,
    pub currently_visible: bool,
}

impl Tile {
    pub fn empty() -> Self {
        Tile {
            blocked: false,
            block_sight: false,
            visible: false,
            currently_visible: false,
        }
    }

    pub fn wall() -> Self {
        Tile {
            blocked: true,
            block_sight: true,
            visible: false,
            currently_visible: false,
        }
    }
}

pub type Map = Vec<Vec<Tile>>;

pub fn make_map(objects: &mut Vec<Object>, level: u32) -> Map {
    let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];

    let mut rooms = vec![];

    for _ in 0..MAX_ROOMS {
        // random width and height
        let w = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        let h = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        // random position without going out of the boundaries of the map
        let x = rand::thread_rng().gen_range(1, MAP_WIDTH - w);
        let y = rand::thread_rng().gen_range(1, MAP_HEIGHT - h);

        let new_room = Rect::new(x, y, w, h);
        object_gen::spawn(new_room, objects, &map, level);

        let failed = rooms
            .iter()
            .any(|other_room| new_room.intersects_with(other_room));

        if !failed {
            map_gen::create_room(new_room, &mut map);

            // center coordinates of the new room, will be useful later
            let (new_x, new_y) = new_room.center();

            if rooms.is_empty() {
                // this is the first room, where the player starts at
                objects[0].x = new_x;
                objects[0].y = new_y;
            } else {
                let (prev_x, prev_y) = rooms[rooms.len() - 1].center();

                // toss a coin (random bool value -- either true or false)
                if rand::random() {
                    // first move horizontally, then vertically
                    map_gen::create_h_tunnel(prev_x, new_x, prev_y, &mut map);
                    map_gen::create_v_tunnel(prev_y, new_y, new_x, &mut map);
                } else {
                    // first move vertically, then horizontally
                    map_gen::create_v_tunnel(prev_y, new_y, prev_x, &mut map);
                    map_gen::create_h_tunnel(prev_x, new_x, new_y, &mut map);
                }
            }
        }

        rooms.push(new_room);
    }

    let (mut last_room_x, mut last_room_y) = rooms[rooms.len() - 1].center();

    while is_blocked(last_room_x, last_room_y, &map, objects) {
        last_room_y += rand::thread_rng().gen_range(-1, 1);
        last_room_x += rand::thread_rng().gen_range(-1, 1);

        if last_room_x > MAP_WIDTH || last_room_x < 0 || last_room_y < 0 || last_room_y > MAP_HEIGHT
        {
            let (x, y) = rooms[rooms.len() - rand::thread_rng().gen_range(1, 4)].center();
            last_room_x = x;
            last_room_y = y;
        }
    }

    let stairs = Object::new(
        last_room_x,
        last_room_y,
        '>',
        pancurses::COLOR_RED,
        true,
        "stairs",
        false,
    );

    objects.push(stairs);

    map
}

pub fn is_blocked(x: i32, y: i32, map: &Map, objects: &[Object]) -> bool {
    // first test the map tile
    if map[x as usize][y as usize].blocked {
        return true;
    }
    // now check for any blocking objects
    objects
        .iter()
        .any(|object| object.blocks && object.pos() == (x, y))
}
