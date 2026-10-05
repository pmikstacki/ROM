<script lang="ts">
  import type {
    ResourceDescriptor,
    WireValue,
    Shape,
  } from "../client/types.ts";
  import type { FilterDraft, FilterRule } from "./types.ts";
  import {
    conjunctionRules,
    filterOperators,
    queryFromDraft,
  } from "./translation.ts";
  import { findRenderer } from "../renderers/registry.ts";
  import { defaultValue } from "../renderers/default-value.ts";
  import ValueEditor from "../renderers/ValueEditor.svelte";
  import SelectAdapter from "../renderers/SelectAdapter.svelte";
  import { Button } from "../components/ui/button/index.js";
  import Rule from "./vendor/Rule.svelte";
  let {
    descriptor,
    draft,
    onchange,
    disabled,
    compact = false,
    onvalidity = () => {},
  }: {
    descriptor: ResourceDescriptor;
    draft: FilterDraft;
    onchange: (next: FilterDraft) => void;
    disabled: boolean;
    compact?: boolean;
    onvalidity?: (error: string) => void;
  } = $props();
  let childErrors = $state<Record<number, string>>({});
  // Private identity follows a rule through our controlled value updates. An
  // external replacement starts a new editor; order/limit edits retain it.
  const identities = new WeakMap<FilterRule, number>();
  let nextIdentity = 0;
  function ruleId(rule: FilterRule): number {
    let id = identities.get(rule);
    if (id === undefined) {
      id = nextIdentity++;
      identities.set(rule, id);
    }
    return id;
  }
  const displayed = $derived.by(() => {
    try {
      return conjunctionRules(draft.rules);
    } catch {
      return [];
    }
  });
  const invalid = $derived.by(() => {
    try {
      queryFromDraft(descriptor, draft);
      for (const rule of displayed) {
        const field = descriptor.fields.find((f) => f.name === rule.field);
        if (
          field?.codec &&
          !findRenderer(field.codec) &&
          rule.filter !== "absent" &&
          rule.filter !== "present"
        )
          return `No renderer is registered for codec ${field.codec.name} version ${field.codec.version}.`;
      }
      return (
        displayed.map((rule) => childErrors[ruleId(rule)]).find(Boolean) ?? ""
      );
    } catch (error) {
      return error instanceof Error ? error.message : "Invalid filters.";
    }
  });
  $effect(() => {
    const active = new Set(displayed.map(ruleId));
    const entries = Object.entries(childErrors);
    if (entries.some(([id]) => !active.has(Number(id)))) {
      childErrors = Object.fromEntries(
        entries.filter(([id]) => active.has(Number(id))),
      );
    }
  });
  $effect(() => {
    onvalidity(invalid);
  });
  function replace(next: FilterRule[]) {
    if (!disabled) onchange({ ...draft, rules: { glue: "and", rules: next } });
  }
  function update(index: number, next: FilterRule) {
    if (disabled) return;
    const rules = [...displayed];
    // Preserve the same identity after a parent stores the rule in deep state.
    const controlled = $state(next);
    identities.set(controlled, ruleId(rules[index]));
    rules[index] = controlled;
    replace(rules);
  }
  function clearError(index: number) {
    const next = { ...childErrors };
    delete next[ruleId(displayed[index])];
    childErrors = next;
  }
  function chooseField(index: number, name: string) {
    const field = descriptor.fields.find((f) => f.name === name);
    if (!field || disabled) return;
    clearError(index);
    update(index, {
      field: name,
      filter: "equal",
      value: defaultValue(field.shape),
    });
  }
  function chooseOperator(index: number, operator: string) {
    if (disabled) return;
    const rule = displayed[index];
    const field = descriptor.fields.find((f) => f.name === rule.field);
    if (!field) return;
    clearError(index);
    update(index, {
      ...rule,
      filter: operator,
      value:
        operator === "absent" || operator === "present"
          ? null
          : rule.filter === "absent" || rule.filter === "present"
            ? defaultValue(field.shape)
            : rule.value,
    });
  }
  function remove(index: number) {
    if (disabled) return;
    clearError(index);
    replace(displayed.filter((_, i) => i !== index));
  }
  function nullable(shape: Shape): boolean {
    return (
      shape.type === "nullable" ||
      (shape.type === "optional" && nullable(shape.value))
    );
  }
  function valueMode(index: number, mode: string) {
    const rule = displayed[index];
    const field = descriptor.fields.find((f) => f.name === rule.field);
    if (!field || disabled) return;
    clearError(index);
    update(index, {
      ...rule,
      value: mode === "null" ? null : defaultValue(field.shape),
    });
  }
  function add() {
    const first = descriptor.fields[0];
    if (first && !disabled)
      replace([
        ...displayed,
        {
          field: first.name,
          filter: "equal",
          value: defaultValue(first.shape),
        },
      ]);
  }
</script>

<div class={compact ? "space-y-3" : "space-y-4"}>
  {#if displayed.length === 0 && !invalid}<p
      class="text-sm text-muted-foreground"
    >
      No conditions. All authorized Resources.
    </p>{/if}
  {#each displayed as rule, index (ruleId(rule))}
    {@const field = descriptor.fields.find((f) => f.name === rule.field)}
    <Rule
      ordinal={index + 1}
      fieldLabel={field?.name ?? rule.field}
      operatorLabel={filterOperators(
        field ?? { name: rule.field, shape: { type: "string" } },
      ).find((o) => o.value === rule.filter)?.label ?? rule.filter}
      {disabled}
      onremove={() => remove(index)}
    >
      <div class="grid min-w-0 gap-2">
        <SelectAdapter
          label={`Filter ${index + 1} field`}
          value={rule.field}
          options={descriptor.fields.map((f) => ({
            value: f.name,
            label: f.name,
          }))}
          onchange={(name) => chooseField(index, name)}
          {disabled}
        />
        {#if field}<SelectAdapter
            label={`Filter ${index + 1} operator`}
            value={rule.filter}
            options={filterOperators(field)}
            onchange={(operator) => chooseOperator(index, operator)}
            {disabled}
          />
          {#if rule.filter !== "absent" && rule.filter !== "present"}
            {#if nullable(field.shape)}<SelectAdapter
                label={`Filter ${index + 1} value mode`}
                value={rule.value === null ? "null" : "value"}
                options={[
                  { value: "value", label: "Value" },
                  { value: "null", label: "Null" },
                ]}
                onchange={(mode) => valueMode(index, mode)}
                {disabled}
              />{/if}
            {#if rule.value !== null}{#key `${rule.field}:${rule.filter}`}<ValueEditor
                  shape={field.shape}
                  codec={field.codec}
                  enumLabels={field.enum_labels}
                  codecWrappers={field.codec_wrappers ?? []}
                  value={rule.value}
                  label={`Filter ${index + 1}`}
                  readonly={disabled}
                  onchange={(value: WireValue) =>
                    update(index, { ...rule, value })}
                  onerror={(error) => {
                    childErrors = { ...childErrors, [ruleId(rule)]: error };
                  }}
                />{/key}{/if}
          {/if}
        {/if}
      </div>
    </Rule>
    {#if index < displayed.length - 1}<div
        class="text-center text-xs font-medium text-muted-foreground"
      >
        AND
      </div>{/if}
  {/each}
  <Button
    type="button"
    variant="outline"
    size="sm"
    disabled={disabled ||
      displayed.length >= 32 ||
      descriptor.fields.length === 0 ||
      (!!invalid && displayed.length === 0)}
    onclick={add}>Add condition</Button
  >
  {#if invalid}<p role="alert" class="text-sm text-destructive">
      {invalid}
    </p>{/if}
</div>
