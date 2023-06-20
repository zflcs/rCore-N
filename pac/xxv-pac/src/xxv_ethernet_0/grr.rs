#[doc = "Register `grr` reader"]
pub struct R(crate::R<GRR_SPEC>);
impl core::ops::Deref for R {
    type Target = crate::R<GRR_SPEC>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<crate::R<GRR_SPEC>> for R {
    #[inline(always)]
    fn from(reader: crate::R<GRR_SPEC>) -> Self {
        R(reader)
    }
}
#[doc = "Register `grr` writer"]
pub struct W(crate::W<GRR_SPEC>);
impl core::ops::Deref for W {
    type Target = crate::W<GRR_SPEC>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl core::ops::DerefMut for W {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl From<crate::W<GRR_SPEC>> for W {
    #[inline(always)]
    fn from(writer: crate::W<GRR_SPEC>) -> Self {
        W(writer)
    }
}
#[doc = "Field `ctl_gt_reset_all` reader - "]
pub type CTL_GT_RESET_ALL_R = crate::BitReader<CTL_GT_RESET_ALL_A>;
#[doc = "\n\nValue on reset: 0"]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CTL_GT_RESET_ALL_A {
    #[doc = "0: `0`"]
    DISABLE = 0,
    #[doc = "1: `1`"]
    ENABLE = 1,
}
impl From<CTL_GT_RESET_ALL_A> for bool {
    #[inline(always)]
    fn from(variant: CTL_GT_RESET_ALL_A) -> Self {
        variant as u8 != 0
    }
}
impl CTL_GT_RESET_ALL_R {
    #[doc = "Get enumerated values variant"]
    #[inline(always)]
    pub fn variant(&self) -> CTL_GT_RESET_ALL_A {
        match self.bits {
            false => CTL_GT_RESET_ALL_A::DISABLE,
            true => CTL_GT_RESET_ALL_A::ENABLE,
        }
    }
    #[doc = "Checks if the value of the field is `DISABLE`"]
    #[inline(always)]
    pub fn is_disable(&self) -> bool {
        *self == CTL_GT_RESET_ALL_A::DISABLE
    }
    #[doc = "Checks if the value of the field is `ENABLE`"]
    #[inline(always)]
    pub fn is_enable(&self) -> bool {
        *self == CTL_GT_RESET_ALL_A::ENABLE
    }
}
#[doc = "Field `ctl_gt_reset_all` writer - "]
pub type CTL_GT_RESET_ALL_W<'a, const O: u8> =
    crate::BitWriter<'a, GRR_SPEC, O, CTL_GT_RESET_ALL_A>;
impl<'a, const O: u8> CTL_GT_RESET_ALL_W<'a, O> {
    #[doc = "`0`"]
    #[inline(always)]
    pub fn disable(self) -> &'a mut W {
        self.variant(CTL_GT_RESET_ALL_A::DISABLE)
    }
    #[doc = "`1`"]
    #[inline(always)]
    pub fn enable(self) -> &'a mut W {
        self.variant(CTL_GT_RESET_ALL_A::ENABLE)
    }
}
#[doc = "Field `ctl_gt_rx_reset` reader - "]
pub type CTL_GT_RX_RESET_R = crate::BitReader<CTL_GT_RX_RESET_A>;
#[doc = "\n\nValue on reset: 0"]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CTL_GT_RX_RESET_A {
    #[doc = "0: `0`"]
    DISABLE = 0,
    #[doc = "1: `1`"]
    ENABLE = 1,
}
impl From<CTL_GT_RX_RESET_A> for bool {
    #[inline(always)]
    fn from(variant: CTL_GT_RX_RESET_A) -> Self {
        variant as u8 != 0
    }
}
impl CTL_GT_RX_RESET_R {
    #[doc = "Get enumerated values variant"]
    #[inline(always)]
    pub fn variant(&self) -> CTL_GT_RX_RESET_A {
        match self.bits {
            false => CTL_GT_RX_RESET_A::DISABLE,
            true => CTL_GT_RX_RESET_A::ENABLE,
        }
    }
    #[doc = "Checks if the value of the field is `DISABLE`"]
    #[inline(always)]
    pub fn is_disable(&self) -> bool {
        *self == CTL_GT_RX_RESET_A::DISABLE
    }
    #[doc = "Checks if the value of the field is `ENABLE`"]
    #[inline(always)]
    pub fn is_enable(&self) -> bool {
        *self == CTL_GT_RX_RESET_A::ENABLE
    }
}
#[doc = "Field `ctl_gt_rx_reset` writer - "]
pub type CTL_GT_RX_RESET_W<'a, const O: u8> = crate::BitWriter<'a, GRR_SPEC, O, CTL_GT_RX_RESET_A>;
impl<'a, const O: u8> CTL_GT_RX_RESET_W<'a, O> {
    #[doc = "`0`"]
    #[inline(always)]
    pub fn disable(self) -> &'a mut W {
        self.variant(CTL_GT_RX_RESET_A::DISABLE)
    }
    #[doc = "`1`"]
    #[inline(always)]
    pub fn enable(self) -> &'a mut W {
        self.variant(CTL_GT_RX_RESET_A::ENABLE)
    }
}
#[doc = "Field `ctl_gt_tx_reset` reader - "]
pub type CTL_GT_TX_RESET_R = crate::BitReader<CTL_GT_TX_RESET_A>;
#[doc = "\n\nValue on reset: 0"]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CTL_GT_TX_RESET_A {
    #[doc = "0: `0`"]
    DISABLE = 0,
    #[doc = "1: `1`"]
    ENABLE = 1,
}
impl From<CTL_GT_TX_RESET_A> for bool {
    #[inline(always)]
    fn from(variant: CTL_GT_TX_RESET_A) -> Self {
        variant as u8 != 0
    }
}
impl CTL_GT_TX_RESET_R {
    #[doc = "Get enumerated values variant"]
    #[inline(always)]
    pub fn variant(&self) -> CTL_GT_TX_RESET_A {
        match self.bits {
            false => CTL_GT_TX_RESET_A::DISABLE,
            true => CTL_GT_TX_RESET_A::ENABLE,
        }
    }
    #[doc = "Checks if the value of the field is `DISABLE`"]
    #[inline(always)]
    pub fn is_disable(&self) -> bool {
        *self == CTL_GT_TX_RESET_A::DISABLE
    }
    #[doc = "Checks if the value of the field is `ENABLE`"]
    #[inline(always)]
    pub fn is_enable(&self) -> bool {
        *self == CTL_GT_TX_RESET_A::ENABLE
    }
}
#[doc = "Field `ctl_gt_tx_reset` writer - "]
pub type CTL_GT_TX_RESET_W<'a, const O: u8> = crate::BitWriter<'a, GRR_SPEC, O, CTL_GT_TX_RESET_A>;
impl<'a, const O: u8> CTL_GT_TX_RESET_W<'a, O> {
    #[doc = "`0`"]
    #[inline(always)]
    pub fn disable(self) -> &'a mut W {
        self.variant(CTL_GT_TX_RESET_A::DISABLE)
    }
    #[doc = "`1`"]
    #[inline(always)]
    pub fn enable(self) -> &'a mut W {
        self.variant(CTL_GT_TX_RESET_A::ENABLE)
    }
}
impl R {
    #[doc = "Bit 0"]
    #[inline(always)]
    pub fn ctl_gt_reset_all(&self) -> CTL_GT_RESET_ALL_R {
        CTL_GT_RESET_ALL_R::new((self.bits & 1) != 0)
    }
    #[doc = "Bit 1"]
    #[inline(always)]
    pub fn ctl_gt_rx_reset(&self) -> CTL_GT_RX_RESET_R {
        CTL_GT_RX_RESET_R::new(((self.bits >> 1) & 1) != 0)
    }
    #[doc = "Bit 2"]
    #[inline(always)]
    pub fn ctl_gt_tx_reset(&self) -> CTL_GT_TX_RESET_R {
        CTL_GT_TX_RESET_R::new(((self.bits >> 2) & 1) != 0)
    }
}
impl W {
    #[doc = "Bit 0"]
    #[inline(always)]
    #[must_use]
    pub fn ctl_gt_reset_all(&mut self) -> CTL_GT_RESET_ALL_W<0> {
        CTL_GT_RESET_ALL_W::new(self)
    }
    #[doc = "Bit 1"]
    #[inline(always)]
    #[must_use]
    pub fn ctl_gt_rx_reset(&mut self) -> CTL_GT_RX_RESET_W<1> {
        CTL_GT_RX_RESET_W::new(self)
    }
    #[doc = "Bit 2"]
    #[inline(always)]
    #[must_use]
    pub fn ctl_gt_tx_reset(&mut self) -> CTL_GT_TX_RESET_W<2> {
        CTL_GT_TX_RESET_W::new(self)
    }
    #[doc = "Writes raw bits to the register."]
    #[inline(always)]
    pub unsafe fn bits(&mut self, bits: u32) -> &mut Self {
        self.0.bits(bits);
        self
    }
}
#[doc = "GT Reset Register\n\nThis register you can [`read`](crate::generic::Reg::read), [`write_with_zero`](crate::generic::Reg::write_with_zero), [`reset`](crate::generic::Reg::reset), [`write`](crate::generic::Reg::write), [`modify`](crate::generic::Reg::modify). See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [grr](index.html) module"]
pub struct GRR_SPEC;
impl crate::RegisterSpec for GRR_SPEC {
    type Ux = u32;
}
#[doc = "`read()` method returns [grr::R](R) reader structure"]
impl crate::Readable for GRR_SPEC {
    type Reader = R;
}
#[doc = "`write(|w| ..)` method takes [grr::W](W) writer structure"]
impl crate::Writable for GRR_SPEC {
    type Writer = W;
    const ZERO_TO_MODIFY_FIELDS_BITMAP: Self::Ux = 0;
    const ONE_TO_MODIFY_FIELDS_BITMAP: Self::Ux = 0;
}
#[doc = "`reset()` method sets grr to value 0"]
impl crate::Resettable for GRR_SPEC {
    const RESET_VALUE: Self::Ux = 0;
}
