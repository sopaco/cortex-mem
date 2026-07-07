# TweetClaw X/Twitter Source Memory

Use this recipe when an OpenClaw agent gathers public X/Twitter context with TweetClaw and stores only reviewed source notes in Cortex Memory.

## Install TweetClaw

```bash
openclaw plugins install @xquik/tweetclaw
openclaw plugins inspect tweetclaw --runtime
openclaw config set tools.alsoAllow '["explore", "tweetclaw"]'
openclaw config set plugins.entries.tweetclaw.config.apiKey "$XQUIK_API_KEY"
```

Keep the key in OpenClaw plugin config or environment-backed secret storage. Do not paste API keys, cookies, DMs, private account data, raw exports, or media files into Cortex memories.

## Collect And Review Sources

Start with `explore` to inspect the TweetClaw endpoint shape, then call `tweetclaw` only for the fields the agent needs. Typical source workflows include:

- Search tweets for a product, competitor, keyword, or campaign.
- Search tweet replies to understand objections, support themes, or follow-up questions.
- Look up public user context when the workflow needs a handle, display name, profile URL, or public metrics.
- Export follower context only when the account owner and workflow allow it.
- Read monitor or webhook events only for monitors the user configured.

Review the results before writing memory. Store conclusions and source references, not raw timelines.

## Store Reviewed Notes

Use one session thread per research run so Cortex Memory can later extract a compact L0/L1/L2 memory trail.

```bash
cortex-mem --config config.toml --tenant acme add \
  --thread tweetclaw-research-2026-05-24 \
  --role user \
  "TweetClaw public X/Twitter search for query 'openclaw memory plugin' found recurring requests for examples that connect social signals to durable agent memory. Sources reviewed: tweet IDs 1234567890 and 2345678901. Decision: add a source-memory recipe and avoid storing raw exports."

cortex-mem --config config.toml --tenant acme session close tweetclaw-research-2026-05-24
```

A reviewed note should usually include:

- Query, endpoint, and capture date.
- Tweet IDs, URLs, author handles, or monitor event IDs.
- The decision, evidence summary, and next action.
- Confidence and any known gaps.

Use `cortex://resources/x-twitter/tweetclaw/...` paths for reusable source summaries and session memory for one-off investigations.

## Search Later

```bash
cortex-mem --config config.toml --tenant acme search \
  "why did we add a TweetClaw source memory example?" \
  --thread tweetclaw-research-2026-05-24 \
  --scope session \
  --limit 5
```

If the answer should outlive the run, promote the final summary into a shared resource note after review.

## Approval Boundaries

TweetClaw can support write workflows such as post tweets, post tweet replies, media upload, and direct messages. Do not store or replay pending write actions from memory without a fresh explicit user approval. Treat Cortex Memory as source context and decision history, not an action queue.

## Verification Checklist

- TweetClaw installs with `openclaw plugins install @xquik/tweetclaw`.
- `openclaw plugins inspect tweetclaw --runtime` reports the plugin runtime as loaded.
- Cortex Memory can add, close, and search the research session.
- Stored notes contain reviewed summaries and source references only.
- No API keys, cookies, DMs, raw exports, or private account material are stored.
