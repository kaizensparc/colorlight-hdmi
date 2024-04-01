# Toplevel external.mk
include $(sort $(wildcard $(BR2_EXTERNAL_COLORLIGHT_PATH)/package/*/*.mk))
