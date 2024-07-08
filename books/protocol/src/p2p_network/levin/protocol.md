# Protocol Messages

This chapter describes protocol messages, and documents the current protocol messages.

## Levin

All protocol messages are in the notification levin format. Altough there are some messages that fall under requests/responses
levin will treat them as notifications


This means requests will NOT set the [expect response bit](./levin.md#expect-response) and responses will set the return code to [`0`](./levin.md#return-code).

## Messages

### Notify New Block

ID: `2001`
