## my_stash

An escrow smart-contract where a user can deposit a given SPL token and lock it for a given time

### Usage

Use 'initialize' to lock tokens for an arbitrary number of seconds.
Though, before that you should prepare a temporary throw-away 'tempTokenAccount' token account with already deposited tokens.

Use 'retrieve' method to get tokens back after a lock time is up.

### TODO:

- Add a proper time forwarding test
- Add 'should fail' tests
- Transfer tokens to the temp token account inside the program

