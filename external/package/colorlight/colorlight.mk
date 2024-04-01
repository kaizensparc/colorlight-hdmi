################################################################################
#
# colorlight
#
################################################################################

COLORLIGHT_VERSION = d94bb9ad69f59298f3a96793ab70366866ec4607
COLORLIGHT_SITE_METHOD = git
COLORLIGHT_SITE = https://github.com/kaizensparc/colorlight-hdmi.git
COLORLIGHT_SUBDIR = colorlight

define COLORLIGHT_INSTALL_INIT_SYSV
	$(INSTALL) -D -m 0755 $(BR2_EXTERNAL_COLORLIGHT_PATH)/package/colorlight/S99colorlight \
		$(TARGET_DIR)/etc/init.d/S99colorlight
endef

$(eval $(cargo-package))
