-- The bag desk is open to everyone (students and teachers), members or not.
-- Keep whatever ID card number was given, linked to a member only when one matches.
ALTER TABLE bag_deposits ADD COLUMN card_number TEXT;
CREATE INDEX bag_deposits_active_card ON bag_deposits(card_number) WHERE status = 'deposited';
