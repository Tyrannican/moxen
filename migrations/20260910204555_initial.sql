create table if not exists retail(
    id integer unique primary key,
    name text not null,
    slug text not null,
    file_id integer not null,
    mod_id integer not null,
    file_name text not null,
);

create table if not exists classic(
    id integer unique primary key,
    name text not null,
    slug text not null,
    file_id integer not null,
    mod_id integer not null,
    file_name text not null,
);

create table if not exists classic_era(
    id integer unique primary key,
    name text not null,
    slug text not null,
    file_id integer not null,
    mod_id integer not null,
    file_name text not null,
);

create table if not exists beta(
    id integer unique primary key,
    name text not null,
    slug text not null,
    file_id integer not null,
    mod_id integer not null,
    file_name text not null,
);

create table if not exists ptr(
    id integer unique primary key,
    name text not null,
    slug text not null,
    file_id integer not null,
    mod_id integer not null,
    file_name text not null,
);

create table if not exists cache(
    mod_id integer not null unique primary key,
    file_id integer not null,
    file blob not null
);
