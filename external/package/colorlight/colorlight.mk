################################################################################
#
# colorlight
#
################################################################################

COLORLIGHT_VERSION = 4d3e6018d08e341d465f8f4b85de437a535a4ed4
COLORLIGHT_SITE_METHOD = git
COLORLIGHT_SITE = https://github.com/kaizensparc/colorlight-hdmi.git
COLORLIGHT_SUBDIR = colorlight

$(eval $(cargo-package))
