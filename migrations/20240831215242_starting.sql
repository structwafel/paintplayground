create table if not exists users
(
    id text primary key not null,
    username text not null,
    xlocation integer not null,
    ylocation integer not null
);

insert into users (id, username, xlocation, ylocation) values ('1', 'admin', 0, 0);
 
create table if not exists passwords
(
    user_id text primary key not null,
    password_hash text not null,
    password_salt text not null,
    FOREIGN key (user_id) references users(id)
);

-- access tokens
create table if not exists jwt_tokens
(
    user_id text primary key not null,
    token text not null,
    FOREIGN key (user_id) references users(id)
);
-- refresh tokens
create table if not exists refresh_tokens
(
    user_id text primary key not null,
    token text not null,
    FOREIGN key (user_id) references users(id)
);