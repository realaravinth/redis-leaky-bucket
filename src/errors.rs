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

#[derive(Debug)]
pub struct CacheError {
    pub msg: String,
}

impl CacheError {
    pub fn new(msg: String) -> Self {
        Self { msg }
    }
}

impl From<String> for CacheError {
    fn from(e: String) -> Self {
        CacheError { msg: e }
    }
}

impl From<&str> for CacheError {
    fn from(e: &str) -> Self {
        CacheError { msg: e.to_string() }
    }
}

impl From<serde_json::Error> for CacheError {
    fn from(e: serde_json::Error) -> Self {
        CacheError { msg: e.to_string() }
    }
}

impl From<CacheError> for redis_module::RedisError {
    fn from(e: CacheError) -> Self {
        redis_module::RedisError::String(e.msg)
    }
}

impl From<CacheError> for redis_module::RedisResult {
    fn from(e: CacheError) -> Self {
        Err(redis_module::RedisError::String(e.msg))
    }
}
