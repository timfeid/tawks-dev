-- A purchase grants its pack once, even if the grant runs twice.
CREATE UNIQUE INDEX IF NOT EXISTS idx_pack_inventory_source_ref
    ON pack_inventory (source, source_ref)
    WHERE source_ref IS NOT NULL;
