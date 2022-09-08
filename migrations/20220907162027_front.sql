-- Add migration script here
CREATE TABLE FRONT (
    Active_Channel INTEGER,
    Channel_ID INTEGER,
    Domain TEXT NOT NULL,
    Token TEXT NOT NULL,
    UserID TEXT NOT NULL,
    TRC INTEGER
)