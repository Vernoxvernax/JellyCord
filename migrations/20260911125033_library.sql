-- Add migration script here
DROP TABLE IF EXISTS LIBRARY;

CREATE TABLE LIBRARY (
  UserID TEXT NOT NULL,
  item_id   TEXT NOT NULL,
  PRIMARY KEY (UserID, item_id)
);
