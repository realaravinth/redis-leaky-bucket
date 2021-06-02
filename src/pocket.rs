/*
 * Copyright (C) 2021  Aravinth Manivannan <realaravinth@batsense.net>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::collections::HashMap;
use std::os::raw::c_void;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use redis_module::native_types::RedisType;
use redis_module::raw::KeyType;
use redis_module::RedisError;
use redis_module::{raw, Context};

pub const PREFIX_COUNTER: &str = "mcaptcha_cache:captcha:";
pub const PREFIX_TIME_POCKET: &str = "mcaptcha_cache:pocket:";

#[derive(Debug, Clone)]
pub struct Pocket {
    timer: Option<u64>,
    pocket_instant: u64,
    decrement: HashMap<String, usize>,
}

#[inline]
/// duration in seconds
fn get_pocket_name(pocket_instant: u64) -> String {
    format!("{}{}", PREFIX_TIME_POCKET, pocket_instant)
}

#[inline]
fn pocket_instant(duration: u64) -> Result<u64, RedisError> {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(val) => Ok(val.as_secs() + duration),
        Err(_) => Err(RedisError::String("SystemTime before UNIX EPOCH!".into())),
    }
}

#[inline]
fn get_captcha_key(name: &str) -> String {
    format!("{}{}", PREFIX_COUNTER, name)
}

impl Pocket {
    /// creates new pocket and sets off timer to go off at `duration`
    pub fn new(ctx: &Context, duration: u64) -> Result<Self, RedisError> {
        let decrement = HashMap::new();

        let pocket_instant = pocket_instant(duration)?;
        let timer = Some(ctx.create_timer(
            Duration::from_secs(duration),
            Self::decrement,
            pocket_instant,
        ));

        let pocket = Pocket {
            timer,
            pocket_instant,
            decrement,
        };
        Ok(pocket)
    }

    /// increments count of key = captcha and registers for auto decrement
    pub fn increment(ctx: &Context, duration: u64, captcha: &str) -> Result<(), RedisError> {
        let captcha_name = get_captcha_key(captcha);
        // increment
        let captcha = ctx.open_key_writable(&captcha_name);
        match captcha.read()? {
            Some(val) => {
                if val.trim().is_empty() {
                    captcha.write("1")?;
                } else {
                    let mut val: usize = val.parse()?;
                    val += 1;
                    captcha.write(&val.to_string())?;
                }
            }
            None => {
                captcha.write("1")?;
            }
        }

        let pocket_instant = pocket_instant(duration)?;
        let pocket_name = get_pocket_name(pocket_instant);

        ctx.log_warning(&format!("Pocket name: {}", &pocket_name));

        // get  pocket
        let pocket = ctx.open_key_writable(&pocket_name);

        match pocket.get_value::<Pocket>(&MCAPTCHA_POCKET_TYPE)? {
            Some(pocket) => match pocket.decrement.get_mut(&captcha_name) {
                Some(count) => *count += 1,
                None => {
                    pocket.decrement.insert(captcha_name, 1);
                }
            },

            None => {
                let mut counter = Pocket::new(ctx, duration)?;
                counter.decrement.insert(captcha_name, 1);
                pocket.set_value(&MCAPTCHA_POCKET_TYPE, counter)?;
                pocket.set_expire(Duration::from_secs(duration + 10))?;
            }
        };

        //    return Ok("OK".into());
        Ok(())
    }

    /// executes when timer goes off. Decrements all registered counts and cleans itself up
    fn decrement(ctx: &Context, pocket_instant: u64) {
        // get  pocket
        let key = ctx.open_key_writable(&get_pocket_name(pocket_instant));

        ctx.log_warning(&format!("Pocket instant: {}", &pocket_instant));
        let val = key.get_value::<Pocket>(&MCAPTCHA_POCKET_TYPE).unwrap();
        ctx.log_warning(&format!("read hashmap "));
        match val {
            Some(pocket) => {
                ctx.log_warning(&format!("entering loop hashmap "));
                for (captcha, count) in pocket.decrement.drain() {
                    ctx.log_warning(&format!(
                        "reading captcha: {} with decr count {}",
                        &captcha, count
                    ));
                    let stored_captcha = ctx.open_key_writable(&captcha);
                    if stored_captcha.key_type() == KeyType::Empty {
                        continue;
                    }

                    let mut stored_count: usize =
                        stored_captcha.read().unwrap().unwrap().parse().unwrap();
                    stored_count -= count;
                    stored_captcha.write(&stored_count.to_string()).unwrap();
                }
            }
            None => {
                ctx.log_warning(&format!("pocket not found, can't decrement"));
            }
        }

        ctx.log_warning(&format!("loop exited"));
        let res = key.delete();

        ctx.log_warning(&format!(
            "enountered error while deleting hashmap: {:?}",
            res
        ));
        //res.unwrap();
    }
}

static MCAPTCHA_POCKET_TYPE: RedisType = RedisType::new(
    "mcaptcha_cache_pocket",
    0,
    raw::RedisModuleTypeMethods {
        version: raw::REDISMODULE_TYPE_METHOD_VERSION as u64,
        rdb_load: None,
        rdb_save: None,
        aof_rewrite: None,
        free: Some(free),

        // Currently unused by Redis
        mem_usage: None,
        digest: None,

        // Aux data
        aux_load: None,
        aux_save: None,
        aux_save_triggers: 0,

        free_effort: None,
        unlink: None,
        copy: None,
        defrag: None,
    },
);

unsafe extern "C" fn free(value: *mut c_void) {
    let val = value as *mut Pocket;
    Box::from_raw(val);
    //    drop(value as *mut Pocket);
}

//fn alloc_set(ctx: &Context, args: Vec<String>) -> RedisResult {
//    let mut args = args.into_iter().skip(1);
//    let key = args.next_string()?;
//    let size = args.next_i64()?;
//
//    ctx.log_debug(format!("key: {}, size: {}", key, size).as_str());
//
//    let key = ctx.open_key_writable(&key);
//
//    match key.get_value::<Pocket>(&MCAPTCHA_POCKET_TYPE)? {
//        Some(value) => {
//            value.data = "B".repeat(size as usize);
//        }
//        None => {
//            let value = MyType {
//                data: "A".repeat(size as usize),
//            };
//
//            key.set_value(&MCAPTCHA_POCKET_TYPE, value)?;
//        }
//    }
//
//    Ok(size.into())
//}

//fn alloc_get(ctx: &Context, args: Vec<String>) -> RedisResult {
//    let mut args = args.into_iter().skip(1);
//    let key = args.next_string()?;
//
//    let key = ctx.open_key(&key);
//
//    let value = match key.get_value::<MyType>(&MCAPTCHA_POCKET_TYPE)? {
//        Some(value) => value.data.as_str().into(),
//        None => ().into(),
//    };
//
//    Ok(value)
//}

//////////////////////////////////////////////////////

//redis_module! {
//    name: "alloc",
//    version: 1,
//    data_types: [
//        MY_REDIS_TYPE,
//    ],
//    commands: [
//        ["alloc.set", alloc_set, "write", 1, 1, 1],
//        ["alloc.get", alloc_get, "readonly", 1, 1, 1],
//    ],
//}