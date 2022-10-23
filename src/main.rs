use std::ffi::{c_void, CStr};

use ash::vk;

struct App {
    entry: ash::Entry,
    physical_device: vk::PhysicalDevice,
    device: ash::Device,
    queue_properties: Vec<vk::QueueFamilyProperties>,
    encode_queue: vk::Queue,
    video_queue_fn: vk::KhrVideoQueueFn,
    video_encode_queue_fn: vk::KhrVideoEncodeQueueFn,
    video_encode_h264_fn: vk::ExtVideoEncodeH264Fn,
}

impl App {
    pub fn new() -> Self {
        unsafe {
            let entry = ash::Entry::load().unwrap();
            let instance = entry
                .create_instance(
                    &vk::InstanceCreateInfo::builder()
                        .application_info(
                            &vk::ApplicationInfo::builder()
                                .api_version(vk::API_VERSION_1_3)
                                .build(),
                        )
                        .build(),
                    None,
                )
                .unwrap();
            let physical_device = instance
                .enumerate_physical_devices()
                .unwrap()
                .into_iter()
                .find(|d| {
                    let p = instance.get_physical_device_properties(*d);
                    p.device_type == vk::PhysicalDeviceType::DISCRETE_GPU
                })
                .unwrap();
            let queue_properties =
                instance.get_physical_device_queue_family_properties(physical_device);
            let encode_queue_index = queue_properties
                .iter()
                .enumerate()
                .find(|(_, p)| p.queue_flags.contains(vk::QueueFlags::VIDEO_ENCODE_KHR))
                .map(|(i, _)| i)
                .unwrap() as u32;

            let device_extensions = vec![
                CStr::from_bytes_with_nul(b"VK_KHR_video_queue\0")
                    .unwrap()
                    .as_ptr(),
                CStr::from_bytes_with_nul(b"VK_KHR_video_encode_queue\0")
                    .unwrap()
                    .as_ptr(),
                CStr::from_bytes_with_nul(b"VK_EXT_video_encode_h264\0")
                    .unwrap()
                    .as_ptr(),
            ];
            let device = instance
                .create_device(
                    physical_device,
                    &vk::DeviceCreateInfo::builder()
                        .queue_create_infos(&[vk::DeviceQueueCreateInfo::builder()
                            .queue_family_index(encode_queue_index)
                            .queue_priorities(&[1.0])
                            .build()])
                        .enabled_extension_names(&device_extensions)
                        .build(),
                    None,
                )
                .unwrap();
            let encode_queue = device.get_device_queue(encode_queue_index, 0);

            entry
                .get_instance_proc_addr(
                    instance.handle(),
                    CStr::from_bytes_with_nul(b"vkGetPhysicalDeviceVideoCapabilitiesKHR\0")
                        .unwrap()
                        .as_ptr(),
                )
                .unwrap();
            let video_queue_fn = vk::KhrVideoQueueFn::load(|name| {
                let addr = match name.to_bytes() {
                    b"vkGetPhysicalDeviceVideoCapabilitiesKHR"
                    | b"vkGetPhysicalDeviceVideoFormatPropertiesKHR" => {
                        entry.get_instance_proc_addr(instance.handle(), name.as_ptr())
                    }
                    _ => instance.get_device_proc_addr(device.handle(), name.as_ptr()),
                };
                match addr {
                    Some(addr) => addr as *const c_void,
                    None => {
                        println!("{:?} not found", name);
                        std::ptr::null()
                    }
                }
            });

            let video_encode_queue_fn = vk::KhrVideoEncodeQueueFn::load(|name| {
                let addr = instance.get_device_proc_addr(device.handle(), name.as_ptr());
                match addr {
                    Some(addr) => addr as *const c_void,
                    None => {
                        println!("{:?} not found", name);
                        std::ptr::null()
                    }
                }
            });

            let video_encode_h264_fn = vk::ExtVideoEncodeH264Fn::load(|name| {
                let addr = instance.get_device_proc_addr(device.handle(), name.as_ptr());
                match addr {
                    Some(addr) => addr as *const c_void,
                    None => {
                        println!("{:?} not found", name);
                        std::ptr::null()
                    }
                }
            });

            let video_profile = vk::VideoProfileKHR::builder()
                .chroma_subsampling(vk::VideoChromaSubsamplingFlagsKHR::TYPE_420)
                .chroma_bit_depth(vk::VideoComponentBitDepthFlagsKHR::TYPE_8)
                .luma_bit_depth(vk::VideoComponentBitDepthFlagsKHR::TYPE_8)
                .video_codec_operation(vk::VideoCodecOperationFlagsKHR::ENCODE_H264_EXT)
                .build();
            let mut video_capabilities = vk::VideoCapabilitiesKHR::default();
            (video_queue_fn.get_physical_device_video_capabilities_khr)(
                physical_device,
                &video_profile,
                &mut video_capabilities,
            )
            .result()
            .unwrap();

            let video_session_info = vk::VideoSessionCreateInfoKHR::builder()
                .video_profile(&video_profile)
                .max_coded_extent(vk::Extent2D::builder().width(1920).height(1080).build())
                .std_header_version(&vk::ExtensionProperties::builder().build())
                .build();
            let mut video_session = vk::VideoSessionKHR::null();
            (video_queue_fn.create_video_session_khr)(
                device.handle(),
                &video_session_info,
                std::ptr::null(),
                &mut video_session,
            )
            .result()
            .unwrap();

            Self {
                entry,
                physical_device,
                device,
                queue_properties,
                encode_queue,
                video_queue_fn,
                video_encode_queue_fn,
                video_encode_h264_fn,
            }
        }
    }
}

fn main() {
    let app = App::new();
    println!("exiting");
}
