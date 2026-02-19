create or replace function ensure_transaction_has_split(target_tx uuid)
returns void as $$
begin
  if exists (
    select 1
    from transactions t
    where t.id = target_tx
      and t.deleted_at is null
      and not exists (
        select 1
        from transaction_splits ts
        where ts.transaction_id = t.id
          and ts.deleted_at is null
      )
  ) then
    raise exception 'transaction must have at least one active split';
  end if;
end;
$$ language plpgsql;

create or replace function enforce_transaction_split_cardinality()
returns trigger as $$
declare
  tx_id uuid;
begin
  tx_id := coalesce(new.transaction_id, old.transaction_id, new.id, old.id);
  perform ensure_transaction_has_split(tx_id);
  return null;
end;
$$ language plpgsql;

create constraint trigger transaction_has_split_on_transactions
after insert or update on transactions
deferrable initially deferred
for each row execute function enforce_transaction_split_cardinality();

create constraint trigger transaction_has_split_on_splits
after insert or update or delete on transaction_splits
deferrable initially deferred
for each row execute function enforce_transaction_split_cardinality();