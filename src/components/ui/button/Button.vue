<script setup lang="ts">
import { computed } from "vue";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

const buttonVariants = cva(
  "inline-flex items-center justify-center rounded-lg px-4 py-2 text-sm font-medium transition-colors disabled:pointer-events-none disabled:opacity-50 focus-visible:focus-ring",
  {
    variants: {
      variant: {
        default: "bg-sky-500 text-white hover:bg-sky-600",
        ghost: "text-slate-600 hover:bg-slate-100 hover:text-slate-950",
      },
    },
    defaultVariants: { variant: "default" },
  },
);

type ButtonVariants = VariantProps<typeof buttonVariants>;
const props = withDefaults(
  defineProps<{
    variant?: ButtonVariants["variant"];
    class?: string;
    type?: "button" | "submit" | "reset";
  }>(),
  { variant: "default", type: "button" },
);
const classes = computed(() => cn(buttonVariants({ variant: props.variant }), props.class));
</script>

<template>
  <button :type="type" :class="classes">
    <slot />
  </button>
</template>
