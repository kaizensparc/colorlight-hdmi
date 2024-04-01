################################################################################
#
# colorlight
#
################################################################################

COLORLIGHT_VERSION = c0ad1afd81b89b03166158c43070b57b3ef389a0
COLORLIGHT_SITE_METHOD = git
COLORLIGHT_SITE = https://github.com/kaizensparc/colorlight-hdmi.git
COLORLIGHT_SUBDIR = colorlight

define COLORLIGHT_INSTALL_INIT_SYSV
	$(INSTALL) -D -m 0755 $(BR2_EXTERNAL_COLORLIGHT_PATH)/package/colorlight/S99colorlight \
		$(TARGET_DIR)/etc/init.d/S99colorlight
endef

$(eval $(cargo-package))
