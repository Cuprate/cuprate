# HTTP methods
`monerod` endpoints supports multiple [HTTP methods](https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods)
that do not necessarily make sense.

For example:
```bash
curl \
	http://127.0.0.1:18081/get_limit \
	-H 'Content-Type: application/json' \
	--request DELETE
```
This is sending an HTTP `DELETE` request, which should be a `GET`.

`monerod` will respond to this the same as `GET`, `POST`, `PUT`, and `TRACE`.

## Cuprate's behavior
> TODO: decide allowed HTTP methods for Cuprate <https://github.com/Cuprate/cuprate/pull/233#discussion_r1700934928>.