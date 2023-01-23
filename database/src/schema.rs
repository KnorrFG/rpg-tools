#![cfg_attr(rustfmt, rustfmt_skip)]

pub const CREATE_STMT: &str = 
"CREATE TABLE nodes (
    name text not null,
    type text not null,
    meta text,
    data blob not null
);

CREATE TABLE links (
    left int not null,
    right int not null,
    type text not null,
    data blob
);";
