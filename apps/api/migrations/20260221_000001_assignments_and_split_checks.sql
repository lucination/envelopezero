create table if not exists category_assignments (
  id uuid primary key default gen_random_uuid(),
  pillid text unique not null default gen_pillid(),
  user_id uuid not null references users(id) on delete cascade,
  user_pillid text not null,
  budget_id uuid not null references budgets(id) on delete cascade,
  budget_pillid text not null,
  category_id uuid not null references categories(id) on delete cascade,
  category_pillid text not null,
  month date not null,
  amount bigint not null,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  deleted_at timestamptz
);

create unique index if not exists category_assignments_key
  on category_assignments(user_id, category_id, month)
  where deleted_at is null;

alter table transaction_splits
  add constraint transaction_splits_non_negative check (inflow >= 0 and outflow >= 0),
  add constraint transaction_splits_single_direction check (
    (inflow = 0 and outflow > 0) or
    (outflow = 0 and inflow > 0)
  );
