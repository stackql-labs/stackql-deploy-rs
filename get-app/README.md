# get-stackql-deploy.io

Cloudflare Worker that backs `https://get-stackql-deploy.io`. It detects the
calling platform and points the caller at the correct `stackql-deploy` release
asset on GitHub.

Behaviour (unchanged from the previous Deno Deploy app):

- `GET /` - reads the `User-Agent` and `302`-redirects to the matching release
  asset (`windows-x86_64.zip`, `macos-universal.tar.gz`, or
  `linux-x86_64.tar.gz`). Keeps the
  `curl -L https://get-stackql-deploy.io | tar xzf -` one-liner working.
- `GET /install.sh` (and `/install`) - returns a POSIX `sh` installer that runs
  `uname` client-side to pick the right OS + arch asset. This is what CLI users
  (curl/wget) hit, since their User-Agent carries no OS.
- Any other path - `301`-redirects to `https://stackql-deploy.io`.

## Develop

```sh
npm install
npm run dev        # wrangler dev - serves on http://localhost:8787
```

Test locally:

```sh
curl -A "curl/8.4.0" http://localhost:8787/install.sh
curl -sI -A "Mozilla/5.0 (Macintosh)" http://localhost:8787/   # -> 302 to macos asset
curl -sI -A "curl/8.4.0" http://localhost:8787/                # -> 302 to linux asset
```

## Deploy

One-time auth (uses your Cloudflare login):

```sh
npx wrangler login
```

Deploy:

```sh
npm run deploy     # wrangler deploy
```

`wrangler.toml` uses a `custom_domain` route for `get-stackql-deploy.io`. On the
first deploy Wrangler creates and manages the proxied DNS record for the apex
automatically - no manual DNS entry required. The `get-stackql-deploy.io` zone
must already exist in the target Cloudflare account.

Tail live logs:

```sh
npm run tail
```

## Cutover from Deno Deploy

The zone is already on Cloudflare, so cutover is just pointing the apex at the
Worker instead of Deno Deploy.

1. Authenticate and deploy the Worker:

   ```sh
   npm install
   npx wrangler login
   npm run deploy
   ```

   Confirm the build output reports the route
   `get-stackql-deploy.io (custom domain)`.

2. In the Cloudflare dashboard for the `get-stackql-deploy.io` zone, check DNS:
   - The previous setup pointed the apex at Deno Deploy (a `CNAME` to
     `<project>.deno.dev`, or `A`/`AAAA` records). Wrangler's custom-domain route
     adds its own managed record for the Worker. If a stale Deno record remains
     and blocks the custom domain from attaching, remove the old Deno
     `CNAME`/`A`/`AAAA` record for the apex, then re-run `npm run deploy`.

3. Verify the live site once DNS propagates (usually seconds on Cloudflare):

   ```sh
   curl -sI -A "curl/8.4.0" https://get-stackql-deploy.io/ | grep -i location
   curl -fsSL https://get-stackql-deploy.io/install.sh | head -5
   curl -L https://get-stackql-deploy.io | tar tzf - | head        # full one-liner
   ```

   Confirm responses are served by Cloudflare (response header
   `server: cloudflare`) and not Deno Deploy.

4. Decommission the Deno Deploy project once verified: delete or pause it in the
   Deno Deploy dashboard so it no longer bills or risks serving stale content.
   The previous Deno source lives in git history if you ever need it.

No consumers need changing - `get-stackql-deploy.io`, `/install.sh`, and the
`curl | tar` one-liner all keep the same URLs and behaviour.
