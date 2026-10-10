<script lang="ts">
  import { ConversationLayout } from "rom-studio/ui/components";
  import { Button, Textarea } from "rom-studio";
  import { attachFollowLatest } from "rom-studio/ui";
  let bodyRef = $state<HTMLDivElement | null>(null);
  let draft = $state("");
  let expanded = $state(false);
  let messages = $state(12);
  let sent = $state(0);
  let following = $state(true);
  let follow: ReturnType<typeof attachFollowLatest> | undefined;
  $effect(() => {
    if (!bodyRef) return;
    follow = attachFollowLatest(bodyRef, {
      thresholdPx: 20,
      reducedMotion: matchMedia("(prefers-reduced-motion: reduce)").matches,
      onFollowing: (value) => (following = value),
    });
    follow.resume();
    return () => follow?.dispose();
  });
  $effect(() => {
    messages;
    follow?.notifyContent();
  });
  function submit() {
    if (!draft.trim()) return;
    sent++;
    draft = "";
  }
</script>

<main>
  <ConversationLayout
    label="Conversation"
    bodyLabel="Messages"
    bind:bodyRef
    bind:expanded
    expansion={{
      expandLabel: "Expand conversation",
      collapseLabel: "Collapse conversation",
    }}
  >
    {#snippet header()}
      <h1>Conversation</h1>
      <Button onclick={() => messages++}>Append message</Button>
      <Button onclick={() => follow?.resume()}>Resume latest</Button>
      <output aria-label="Following latest">{following ? "yes" : "no"}</output>
    {/snippet}
    {#snippet history()}
      <details>
        <summary>Saved history</summary>
        <ol>
          {#each Array(50) as _, index}<li>
              <button
                >Saved conversation {index + 1}: a readable long title</button
              >
            </li>{/each}
        </ol>
      </details>
    {/snippet}
    {#snippet body()}
      <ol aria-label="Messages">
        {#each Array(messages) as _, index}<li class="message">
            Message {index + 1}. {"Long content ".repeat(35)}
          </li>{/each}
      </ol>
    {/snippet}
    {#snippet footer()}
      <form
        onsubmit={(event) => {
          event.preventDefault();
          submit();
        }}
      >
        <label for="draft">Message</label>
        <Textarea
          id="draft"
          bind:value={draft}
          rows={1}
          class="min-h-11 max-h-24"
          onkeydown={(event) => {
            if (
              event.key === "Enter" &&
              !event.shiftKey &&
              !event.isComposing
            ) {
              event.preventDefault();
              submit();
            }
          }}
        />
        <Button type="submit" disabled={!draft.trim()}>Send message</Button>
        <output aria-label="Submitted messages">{sent}</output>
      </form>
    {/snippet}
  </ConversationLayout>
</main>

<style>
  :global(body) {
    margin: 0;
  }
  main {
    height: 100dvh;
    padding: 12px;
    box-sizing: border-box;
    display: flex;
    justify-content: center;
  }
  h1 {
    font-weight: 600;
  }
  form {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 6px;
  }
  label {
    grid-column: 1 / -1;
  }
  output {
    font-size: 12px;
  }
  .message {
    padding: 20px;
    border-bottom: 1px solid;
    overflow-wrap: anywhere;
  }
</style>
