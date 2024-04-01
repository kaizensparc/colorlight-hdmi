################################################################################
#
# colorlight
#
################################################################################

COLORLIGHT_VERSION = 0.1.0
COLORLIGHT_SITE_METHOD = local
COLORLIGHT_SITE = $(BR2_EXTERNAL_COLORLIGHT_PATH)/package/colorlight/sources
COLORLIGHT_LICENSE = MIT
COLORLIGHT_LICENSE_FILES = LICENSE-MIT

define COLORLIGHT_INSTALL_INIT_SYSV
	$(INSTALL) -D -m 0755 $(BR2_EXTERNAL_BRASHAT_PATH)/package/colorlight/S99colorlight \
		$(TARGET_DIR)/etc/init.d/S99colorlight
endef

$(eval $(cargo-package))
