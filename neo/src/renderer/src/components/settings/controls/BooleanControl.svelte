<script lang="ts">
  import { Checkbox } from '$lib/components/ui/checkbox'

  interface Props {
    value: boolean
    onupdate: (value: boolean) => void
  }

  let { value, onupdate }: Props = $props()
</script>

<div class="boolean-control">
  <Checkbox
    checked={value}
    onCheckedChange={(checked) => onupdate(Boolean(checked))}
  />
</div>

<style>
  .boolean-control {
    display: flex;
    align-items: center;
  }
</style>
