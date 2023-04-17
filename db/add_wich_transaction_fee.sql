ALTER TABLE tx
ADD COLUMN wich_transaction_fee INT UNSIGNED;

ALTER TABLE tx
ADD CONSTRAINT fk_fee_transaction FOREIGN KEY (wich_transaction_fee) REFERENCES fee_transaction (id);

UPDATE tx t
SET t.wich_transaction_fee  = (SELECT ft.id FROM fee_transaction ft WHERE Date(ft.`time`) = Date(t.`time`))
WHERE t.state = 'PROCESSED' and t.wich_transaction_fee is NULL ;