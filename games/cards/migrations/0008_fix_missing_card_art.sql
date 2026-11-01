-- These image links pointed at files that were never uploaded.
UPDATE cards SET image_url = 'https://cdn.tawks.app/cards/terravore_1774307412.png' WHERE card_id = 'terravore';
UPDATE cards SET image_url = 'https://cdn.tawks.app/cards/sol_core_1774307376.png' WHERE card_id = 'sol_core';
UPDATE cards SET image_url = 'https://cdn.tawks.app/cards/xylo_commander_1772751873.png' WHERE card_id = 'xylo_operator';
-- No art exists yet for these; the card draws without it instead of a broken image.
UPDATE cards SET image_url = NULL WHERE card_id IN ('neon_spike', 'signal_stabilizer', 'chrome_spike');
