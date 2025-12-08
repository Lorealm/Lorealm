<template>
  <div class="tooltip-wrapper" ref="wrapper">
    <div 
      ref="trigger"
      @mouseenter="onMouseEnter"
      @mouseleave="onMouseLeave"
    >
      <slot name="trigger"></slot>
    </div>
    
    <div 
      v-if="showTooltip"
      class="smart-tooltip"
      :class="{ 'tooltip-positioned': isPositioned }"
      :style="tooltipStyle"
      ref="tooltip"
    >
      <div class="tooltip-content">
        <slot name="content"></slot>
      </div>
      <div class="tooltip-arrow" :style="arrowStyle"></div>
    </div>
  </div>
</template>

<script>
export default {
  name: 'SmartTooltip',
  props: {
    disabled: {
      type: Boolean,
      default: false
    },
    delay: {
      type: Number,
      default: 0
    },
    spacing: {
      type: Number,
      default: 8
    },
    verticalOffset: {
      type: Number,
      default: 0
    }
  },
  data() {
    return {
      showTooltip: false,
      isPositioned: false,
      tooltipStyle: {},
      arrowStyle: {},
      showTimeout: null
    }
  },
  methods: {
    onMouseEnter() {
      if (this.disabled) return
      
      if (this.delay > 0) {
        this.showTimeout = setTimeout(() => {
          this.showTooltip = true
          this.isPositioned = false
          this.$nextTick(() => {
            this.positionTooltip()
          })
        }, this.delay)
      } else {
        this.showTooltip = true
        this.isPositioned = false
        this.$nextTick(() => {
          this.positionTooltip()
        })
      }
    },
    
    onMouseLeave() {
      if (this.showTimeout) {
        clearTimeout(this.showTimeout)
        this.showTimeout = null
      }
      this.showTooltip = false
      this.isPositioned = false
    },
    
    positionTooltip() {
      if (!this.$refs.tooltip || !this.$refs.trigger) return
      
      const trigger = this.$refs.trigger
      const tooltip = this.$refs.tooltip
      const triggerRect = trigger.getBoundingClientRect()
      const tooltipRect = tooltip.getBoundingClientRect()
      const viewport = {
        width: window.innerWidth,
        height: window.innerHeight
      }
      
      const spacing = this.spacing
      
      // Always position tooltip above the trigger (fixed position)
      // Add verticalOffset to increase distance from trigger
      const tooltipTop = triggerRect.top - tooltipRect.height - spacing - this.verticalOffset
      
      // Calculate horizontal position to center the tooltip
      const idealLeft = triggerRect.left + (triggerRect.width - tooltipRect.width) / 2
      
      // Ensure tooltip doesn't go off screen horizontally
      let finalLeft = idealLeft
      if (idealLeft < spacing) {
        finalLeft = spacing
      } else if (idealLeft + tooltipRect.width > viewport.width - spacing) {
        finalLeft = viewport.width - tooltipRect.width - spacing
      }
      
      // Calculate arrow position relative to tooltip
      const triggerCenter = triggerRect.left + triggerRect.width / 2
      const tooltipLeftEdge = finalLeft
      const arrowLeft = triggerCenter - tooltipLeftEdge
      
      // Constrain arrow to stay within tooltip bounds (with some padding)
      const arrowPadding = 12
      const constrainedArrowLeft = Math.max(
        arrowPadding,
        Math.min(arrowLeft, tooltipRect.width - arrowPadding)
      )
      
      this.tooltipStyle = {
        position: 'fixed',
        top: `${tooltipTop}px`,
        left: `${finalLeft}px`,
        zIndex: 1000
      }
      
      this.arrowStyle = {
        position: 'absolute',
        bottom: '-6px',
        left: `${constrainedArrowLeft}px`,
        transform: 'translateX(-50%)',
        width: '0',
        height: '0',
        borderLeft: '6px solid transparent',
        borderRight: '6px solid transparent',
        borderTop: '6px solid #1f2937'
      }
      
      // Mark as positioned to show the tooltip
      this.isPositioned = true
    }
  },
  
  beforeUnmount() {
    if (this.showTimeout) {
      clearTimeout(this.showTimeout)
    }
  }
}
</script>

<style scoped>
.tooltip-wrapper {
  position: relative;
  display: inline-block;
}

.smart-tooltip {
  background: #1f2937;
  color: white;
  border-radius: 8px;
  box-shadow: 0 10px 25px rgba(0, 0, 0, 0.2);
  font-size: 12px;
  pointer-events: none;
  opacity: 0;
  transition: opacity 0.2s ease-out;
}

.smart-tooltip.tooltip-positioned {
  opacity: 1;
}

.tooltip-content {
  padding: 12px;
  position: relative;
}
</style>