<script lang="ts">
  import {ConversationLayout as SharedConversationLayout} from "rom-ui/ui/components";
  import type {ComponentProps} from "svelte";
  let {expanded = $bindable(false), bodyRef = $bindable(null), ...props}: ComponentProps<typeof SharedConversationLayout> = $props();
</script>
<SharedConversationLayout {...props} bind:expanded bind:bodyRef />
