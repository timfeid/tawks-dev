<script lang="ts">
  import { AtSign, Paperclip, Send, Smile } from '@lucide/svelte';
  import { Button } from '../../../../../apps/tawks/src/lib/components/ui/button';
  import MessageArea from '../../lib/components/message-area.svelte';
</script>

<div class="bg-background h-[100vh] flex h-16">
  <div class="bg-background group border-t p-4 mt-auto w-full">
    <div class=" flex items-center">
      <Button variant="ghost" size="icon" class="hover:text-primary group-hover:opacity-100">
        <Paperclip class="h-4 w-4" />
      </Button>
      <div class="relative flex-1">
        <MessageArea channel="#tawks-general"></MessageArea>
      </div>
    </div>
  </div>
</div>
