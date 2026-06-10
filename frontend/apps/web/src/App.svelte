<script lang="ts">
 import { App, type PlatformAPI } from "@lorealm/ui";

 const api: PlatformAPI = {
      http: {} as any,
      capabilities: new Set(['placeholder'])
  }
</script>

<App {api} />
