-- Add migration script here
CREATE TABLE JellyCord (
    Active_channel INTEGER,
    Channel_Id INTEGER,
    Endpoint TEXT NOT NULL,
    Token TEXT NOT NULL,
    UserId TEXT NOT NULL,
    LastId TEXT NOT NULL
)