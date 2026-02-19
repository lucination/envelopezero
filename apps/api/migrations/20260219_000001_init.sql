create extension if not exists pgcrypto;

create table if not exists users (
  id uuid primary key default gen_random_uuid(),
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists auth_methods (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references users(id) on delete cascade,
  method_type text not null check (method_type in ('magic_link_email', 'passkey')),
  label text,
  created_at timestamptz not null default now(),
  disabled_at timestamptz
);

create table if not exists user_emails (
  user_id uuid not null references users(id) on delete cascade,
  email text not null,
  verified_at timestamptz,
  primary key (user_id, email)
);

create table if not exists passkey_credentials (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references users(id) on delete cascade,
  credential_id text not null unique,
  public_key text not null,
  sign_count bigint not null default 0,
  transports text[],
  created_at timestamptz not null default now(),
  disabled_at timestamptz
);

create table if not exists passkey_challenges (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references users(id) on delete cascade,
  challenge text not null,
  purpose text not null check (purpose in ('register', 'authenticate')),
  used_at timestamptz,
  created_at timestamptz not null default now(),
  expires_at timestamptz not null
);

create table if not exists magic_link_tokens (
  id uuid primary key default gen_random_uuid(),
  email text not null,
  token_hash text not null,
  consumed_at timestamptz,
  created_at timestamptz not null default now(),
  expires_at timestamptz not null
);

create table if not exists sessions (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references users(id) on delete cascade,
  token_hash text not null unique,
  created_at timestamptz not null default now(),
  expires_at timestamptz not null,
  revoked_at timestamptz
);

create table if not exists email_outbox (
  id uuid primary key default gen_random_uuid(),
  to_email text not null,
  subject text not null,
  body text not null,
  queued_at timestamptz not null default now(),
  sent_at timestamptz
);

create or replace function assert_user_has_auth_method(p_user uuid)
returns void language plpgsql as $$
declare v_count int;
begin
  select count(*) into v_count
  from auth_methods
  where user_id = p_user and disabled_at is null;

  if v_count < 1 then
    raise exception 'user % must have at least one active auth method', p_user;
  end if;
end;
$$;
