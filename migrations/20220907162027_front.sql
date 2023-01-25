-- Add migration script here
CREATE TABLE FRONT (
    Active_Channel INTEGER NOT NULL,
    Channel_ID INTEGER NOT NULL,
    Domain TEXT NOT NULL,
    Token TEXT NOT NULL,
    UserID TEXT NOT NULL
)