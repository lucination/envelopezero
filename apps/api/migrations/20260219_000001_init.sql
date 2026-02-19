create extension if not exists pgcrypto;

create or replace function gen_pillid() returns text as $$
  select lower(encode(gen_random_bytes(16), 'hex'));
$$ language sql volatile;

create table if not exists users (
  id uuid primary key default gen_random_uuid(),
  pillid text unique,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table if not exists auth_methods (
  id uuid primary key default gen_random_uuid(),
  pillid text unique,
  user_id uuid not null references users(id) on delete cascade,
  user_pillid text,
  method_type text not null check (method_type in ('magic_link_email', 'passkey')),
  label text,
  created_at timestamptz not null default now(),
  disabled_at timestamptz
);

create table if not exists user_emails (
  user_id uuid not null references users(id) on delete cascade,
  user_pillid text,
  email text not null unique,
  verified_at timestamptz,
  primary key (user_id, email)
);
create unique index if not exists user_emails_user_pillid_email_key on user_emails(user_pillid, email);

create table if not exists magic_link_tokens (
  id uuid primary key default gen_random_uuid(),
  pillid text unique,
  email text not null,
  token_hash text not null,
  consumed_at timestamptz,
  created_at timestamptz not null default now(),
  expires_at timestamptz not null
);

create table if not exists sessions (
  id uuid primary key default gen_random_uuid(),
  pillid text unique,
  user_id uuid not null references users(id) on delete cascade,
  user_pillid text,
  token_hash text not null unique,
  created_at timestamptz not null default now(),
  expires_at timestamptz not null,
  revoked_at timestamptz
);

create table if not exists email_outbox (
  id uuid primary key default gen_random_uuid(),
  pillid text unique,
  to_email text not null,
  subject text not null,
  body text not null,
  queued_at timestamptz not null default now(),
  sent_at timestamptz
);

create table if not exists budgets (
  id uuid primary key default gen_random_uuid(),
  pillid text unique,
  user_id uuid not null references users(id) on delete cascade,
  user_pillid text,
  name text not null,
  currency_code text not null default 'USD',
  is_default boolean not null default false,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  deleted_at timestamptz
);

create table if not exists accounts (
  id uuid primary key default gen_random_uuid(),
  pillid text unique,
  user_id uuid not null references users(id) on delete cascade,
  user_pillid text,
  budget_id uuid not null references budgets(id) on delete cascade,
  budget_pillid text,
  name text not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  deleted_at timestamptz,
  unique(user_id, budget_id, name)
);

create table if not exists supercategories (
  id uuid primary key default gen_random_uuid(),
  pillid text unique,
  user_id uuid not null references users(id) on delete cascade,
  user_pillid text,
  budget_id uuid not null references budgets(id) on delete cascade,
  budget_pillid text,
  name text not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  deleted_at timestamptz
);

create table if not exists categories (
  id uuid primary key default gen_random_uuid(),
  pillid text unique,
  user_id uuid not null references users(id) on delete cascade,
  user_pillid text,
  budget_id uuid not null references budgets(id) on delete cascade,
  budget_pillid text,
  supercategory_id uuid not null references supercategories(id) on delete cascade,
  supercategory_pillid text,
  name text not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  deleted_at timestamptz
);

create table if not exists transactions (
  id uuid primary key default gen_random_uuid(),
  pillid text unique,
  user_id uuid not null references users(id) on delete cascade,
  user_pillid text,
  budget_id uuid not null references budgets(id) on delete cascade,
  budget_pillid text,
  account_id uuid not null references accounts(id) on delete cascade,
  account_pillid text,
  tx_date date not null,
  payee text,
  memo text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  deleted_at timestamptz
);

create table if not exists transaction_splits (
  id uuid primary key default gen_random_uuid(),
  pillid text unique,
  transaction_id uuid not null references transactions(id) on delete cascade,
  transaction_pillid text,
  category_id uuid not null references categories(id) on delete cascade,
  category_pillid text,
  memo text,
  inflow bigint not null default 0,
  outflow bigint not null default 0,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  deleted_at timestamptz
);

create table if not exists passkey_credentials (
  id uuid primary key default gen_random_uuid(),
  pillid text unique,
  user_id uuid not null references users(id) on delete cascade,
  user_pillid text,
  credential_id text not null unique,
  public_key text not null,
  sign_count bigint not null default 0,
  transports text[],
  created_at timestamptz not null default now(),
  disabled_at timestamptz
);

create table if not exists passkey_challenges (
  id uuid primary key default gen_random_uuid(),
  pillid text unique,
  user_id uuid not null references users(id) on delete cascade,
  user_pillid text,
  challenge text not null,
  purpose text not null check (purpose in ('register', 'authenticate')),
  used_at timestamptz,
  created_at timestamptz not null default now(),
  expires_at timestamptz not null
);

update users set pillid = coalesce(pillid, gen_pillid());
alter table users alter column pillid set default gen_pillid();
alter table users alter column pillid set not null;

update auth_methods set pillid = coalesce(pillid, gen_pillid());
update auth_methods am set user_pillid = u.pillid from users u where am.user_id = u.id and am.user_pillid is null;
alter table auth_methods alter column pillid set default gen_pillid();
alter table auth_methods alter column pillid set not null;
alter table auth_methods alter column user_pillid set not null;

update user_emails ue set user_pillid = u.pillid from users u where ue.user_id = u.id and ue.user_pillid is null;
alter table user_emails alter column user_pillid set not null;

update magic_link_tokens set pillid = coalesce(pillid, gen_pillid());
alter table magic_link_tokens alter column pillid set default gen_pillid();
alter table magic_link_tokens alter column pillid set not null;

update sessions set pillid = coalesce(pillid, gen_pillid());
update sessions s set user_pillid = u.pillid from users u where s.user_id = u.id and s.user_pillid is null;
alter table sessions alter column pillid set default gen_pillid();
alter table sessions alter column pillid set not null;
alter table sessions alter column user_pillid set not null;

update email_outbox set pillid = coalesce(pillid, gen_pillid());
alter table email_outbox alter column pillid set default gen_pillid();
alter table email_outbox alter column pillid set not null;

update budgets set pillid = coalesce(pillid, gen_pillid());
update budgets b set user_pillid = u.pillid from users u where b.user_id = u.id and b.user_pillid is null;
alter table budgets alter column pillid set default gen_pillid();
alter table budgets alter column pillid set not null;
alter table budgets alter column user_pillid set not null;

update accounts set pillid = coalesce(pillid, gen_pillid());
update accounts a set user_pillid = u.pillid from users u where a.user_id = u.id and a.user_pillid is null;
update accounts a set budget_pillid = b.pillid from budgets b where a.budget_id = b.id and a.budget_pillid is null;
alter table accounts alter column pillid set default gen_pillid();
alter table accounts alter column pillid set not null;
alter table accounts alter column user_pillid set not null;
alter table accounts alter column budget_pillid set not null;

update supercategories set pillid = coalesce(pillid, gen_pillid());
update supercategories s set user_pillid = u.pillid from users u where s.user_id = u.id and s.user_pillid is null;
update supercategories s set budget_pillid = b.pillid from budgets b where s.budget_id = b.id and s.budget_pillid is null;
alter table supercategories alter column pillid set default gen_pillid();
alter table supercategories alter column pillid set not null;
alter table supercategories alter column user_pillid set not null;
alter table supercategories alter column budget_pillid set not null;

update categories set pillid = coalesce(pillid, gen_pillid());
update categories c set user_pillid = u.pillid from users u where c.user_id = u.id and c.user_pillid is null;
update categories c set budget_pillid = b.pillid from budgets b where c.budget_id = b.id and c.budget_pillid is null;
update categories c set supercategory_pillid = s.pillid from supercategories s where c.supercategory_id = s.id and c.supercategory_pillid is null;
alter table categories alter column pillid set default gen_pillid();
alter table categories alter column pillid set not null;
alter table categories alter column user_pillid set not null;
alter table categories alter column budget_pillid set not null;
alter table categories alter column supercategory_pillid set not null;

update transactions set pillid = coalesce(pillid, gen_pillid());
update transactions t set user_pillid = u.pillid from users u where t.user_id = u.id and t.user_pillid is null;
update transactions t set budget_pillid = b.pillid from budgets b where t.budget_id = b.id and t.budget_pillid is null;
update transactions t set account_pillid = a.pillid from accounts a where t.account_id = a.id and t.account_pillid is null;
alter table transactions alter column pillid set default gen_pillid();
alter table transactions alter column pillid set not null;
alter table transactions alter column user_pillid set not null;
alter table transactions alter column budget_pillid set not null;
alter table transactions alter column account_pillid set not null;

update transaction_splits set pillid = coalesce(pillid, gen_pillid());
update transaction_splits ts set transaction_pillid = t.pillid from transactions t where ts.transaction_id = t.id and ts.transaction_pillid is null;
update transaction_splits ts set category_pillid = c.pillid from categories c where ts.category_id = c.id and ts.category_pillid is null;
alter table transaction_splits alter column pillid set default gen_pillid();
alter table transaction_splits alter column pillid set not null;
alter table transaction_splits alter column transaction_pillid set not null;
alter table transaction_splits alter column category_pillid set not null;

update passkey_credentials set pillid = coalesce(pillid, gen_pillid());
update passkey_credentials pc set user_pillid = u.pillid from users u where pc.user_id = u.id and pc.user_pillid is null;
alter table passkey_credentials alter column pillid set default gen_pillid();
alter table passkey_credentials alter column pillid set not null;
alter table passkey_credentials alter column user_pillid set not null;

update passkey_challenges set pillid = coalesce(pillid, gen_pillid());
update passkey_challenges pc set user_pillid = u.pillid from users u where pc.user_id = u.id and pc.user_pillid is null;
alter table passkey_challenges alter column pillid set default gen_pillid();
alter table passkey_challenges alter column pillid set not null;
alter table passkey_challenges alter column user_pillid set not null;
